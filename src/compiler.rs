use std::{iter::Peekable, slice::Iter};

use crate::{
    Error,
    lexer::{Token, TokenKind},
    vm::{Instruction, Runtime},
};

pub fn compile(tokens: Vec<Token>, runtime: &mut Runtime) -> Result<Module, Error> {
    let mut compiler = Compiler {
        runtime,
        // globals: Default::default(),
        // field_to_id_map: ahash::HashMap::default(),
        tokens: tokens.iter().peekable(),
        code: vec![],
        constants: vec![],
    };

    while compiler.tokens.peek().is_some() {
        compiler.compile_statement()?;
    }

    compiler.code.push(Instruction::Halt);

    Ok(Module {
        constants: compiler.constants,
        code: compiler.code,
    })
}

#[derive(Debug)]
pub struct Module {
    pub code: Vec<Instruction>,
    pub constants: Vec<f64>,
}

pub struct Compiler<'s> {
    runtime: &'s mut Runtime,
    // globals: HashMap<String, u32>,
    // field_to_id_map: ahash::HashMap<String, u32>,
    tokens: Peekable<Iter<'s, Token>>,
    code: Vec<Instruction>,
    constants: Vec<f64>,
}

impl<'s> Compiler<'s> {
    pub fn consume(&mut self, expected: TokenKind) -> Result<(), Error> {
        if let Some(token) = self.tokens.next() {
            if token.kind == expected {
                Ok(())
            } else {
                Err(Error::UnexpectedTokenExpected(token.kind, expected))
            }
        } else {
            Err(Error::UnexpectedEOFExpected(expected))
        }
    }

    pub fn compile_statement(&mut self) -> Result<(), Error> {
        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Semicolon => {
                    self.tokens.next();
                    continue;
                }

                TokenKind::Nil
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Number
                | TokenKind::Ident
                | TokenKind::String => {
                    self.compile_member()?;
                    self.consume(TokenKind::Semicolon)?;
                    self.code.push(Instruction::Pop);
                }

                TokenKind::End | TokenKind::Else | TokenKind::ElseIf => {
                    break;
                }

                TokenKind::Minus => todo!(),
                TokenKind::If => self.compile_if_stmt()?,
                TokenKind::While => self.compile_while_stmt()?,

                _ => {
                    return Err(Error::UnexpectedToken((*token).clone()));
                }
            }
        }

        Ok(())
    }

    fn compile_while_stmt(&mut self) -> Result<(), Error> {
        self.consume(TokenKind::While)?;

        let start = self.code.len();
        self.compile_member()?;

        let jump = self.code.len();
        self.code.push(Instruction::JmpIfFalse { addr: 0xdead });

        self.consume(TokenKind::Do)?;

        while let Some(token) = self.tokens.peek() {
            if token.kind != TokenKind::End {
                self.compile_statement()?;
            } else {
                break;
            }
        }

        self.consume(TokenKind::End)?;

        let end = self.code.len();
        self.code.push(Instruction::Jmp {
            addr: start as i32 - end as i32 - 1,
        });
        self.code[jump] = Instruction::JmpIfFalse {
            addr: (end - jump) as i32,
        };

        Ok(())
    }

    fn compile_if_stmt(&mut self) -> Result<(), Error> {
        // Consume the IF token.
        // println!("{:?}", self.tokens.next());
        self.consume(TokenKind::If)?;

        self.compile_member()?;

        self.consume(TokenKind::Then)?;

        // Position of last jump instruction emitted by compiler.
        let mut last_jmp_inst = self.code.len();
        self.code.push(Instruction::JmpIfFalse { addr: 0xdead });
        let mut has_else = false;
        // self.code.push(Instruction::Pop); // Pop the condition off the stack.

        /*
        // if
        <Cond>
        JmpIfFalse --+      set prev_branch
        <Pop>        |
        <Stmt>       |
        <...>        |
        Jmp ---------|-+    store in `jumps`
        <Cond> <-----+ |
        // elseif      |
        JmpIfFalse --+ |    set prev_branch
        <Pop>        | |
        <Stmt>       | |
        <...>        | |
        Jmp ---------|-|-+  store in `jumps`
        // else      | | |
        <Stmt> <-----+ | |
        <Stmt>         | |
        <END>  <-------+-+
         */

        // Absolute jumps that need to be patched.
        let mut jumps: Vec<usize> = vec![];

        while let Some(token) = self.tokens.peek() {
            // TODO: get ELSE and ELSEIF's working.
            if token.kind == TokenKind::Else {
                self.consume(TokenKind::Else)?;

                let jump_inst = self.code.len();
                jumps.push(jump_inst);
                self.code.push(Instruction::Jmp { addr: 0xdead_b0b }); // In honor of Bob Nystrom.

                // Update the last jump instruction so that it jumps to this branch.
                self.code[last_jmp_inst] = Instruction::JmpIfFalse {
                    addr: (jump_inst - last_jmp_inst) as i32,
                };

                has_else = true;
            } else if token.kind == TokenKind::ElseIf {
                // Consume the ELSEIF token.
                self.consume(TokenKind::ElseIf)?;

                // Emit an unconditional jump instruction for the previous branch to take.
                let jump_inst = self.code.len();
                // This instruction will need to be updated after we compile all clauses so
                // we store it for later.
                jumps.push(jump_inst);
                // Emit a placeholder instruction that will be updated later.
                self.code.push(Instruction::Jmp { addr: 0xdead_b0b }); // In honor of Bob Nystrom.

                // Update the last jump instruction so that it jumps to this branch.
                self.code[last_jmp_inst] = Instruction::JmpIfFalse {
                    addr: (jump_inst - last_jmp_inst) as i32,
                };

                // Compile the branch condition.
                self.compile_member()?;

                last_jmp_inst = self.code.len();
                // Emit the instruction to skip this block and go to the next.
                self.code.push(Instruction::JmpIfFalse { addr: 0xdead_b0b });
                // Emit instruction to pop condition value off of stack.
                // self.code.push(Instruction::Pop);

                self.consume(TokenKind::Then)?;
            } else if token.kind == TokenKind::End {
                self.consume(TokenKind::End)?;
                break;
            } else {
                self.compile_statement()?;
            }
        }

        let last = self.code.len() as i32;

        if !has_else {
            // Update the last jump instruction so that it jumps to this branch.
            self.code[last_jmp_inst] = Instruction::JmpIfFalse {
                addr: last - last_jmp_inst as i32 - 1,
            };
        }

        let last = self.code.len() as i32;

        // println!("{jumps:?}");

        for jump in jumps {
            let addr = last - jump as i32 - 1;
            self.code[jump] = Instruction::Jmp { addr };
        }

        Ok(())
    }

    fn compile_member(&mut self) -> Result<(), Error> {
        self.compile_atom()?;

        while let Some(token) = self.tokens.peek() {
            if token.kind == TokenKind::Dot {
                self.tokens.next();

                let Some(next_token) = self.tokens.next() else {
                    return Err(Error::UnexpectedEOF);
                };

                if next_token.kind == TokenKind::Ident {
                    let name = next_token.data.clone();

                    // If the next token is an equal sign, then this becomes a store
                    // operation. If the next token is a left parentheses, then this becomes
                    // a method invocation operation.
                    if let Some(token) = self.tokens.peek() {
                        match token.kind {
                            TokenKind::Equal => {
                                // Emit an `IndexSet` instruction.
                                // let name = token.data.clone();
                                self.consume(TokenKind::Equal)?;

                                self.compile_member()?;

                                let id = self.runtime.get_field_index(&name);
                                self.code.push(Instruction::IndexSet { index: id });
                            }
                            TokenKind::LParen => {
                                let sym = self.runtime.get_field_index(&name);

                                // Emit an `Invoke` instruction.
                                self.consume(TokenKind::LParen)?;

                                let mut args = 0u8;
                                while let Some(token) = self.tokens.peek() {
                                    if token.kind == TokenKind::RParen {
                                        break;
                                    } else {
                                        args += 1;
                                        self.compile_member()?;

                                        // Optional trailing comma.
                                        if let Some(token) = self.tokens.peek() {
                                            if token.kind == TokenKind::Comma {
                                                self.consume(TokenKind::Comma)?;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }

                                self.consume(TokenKind::RParen)?;

                                self.code.push(Instruction::Invoke { args, sym });
                            }
                            _ => {
                                let id = self.runtime.get_field_index(&name);
                                self.code.push(Instruction::IndexGet { index: id });

                                continue;
                            }
                        }
                    }
                } else {
                    return Err(Error::UnexpectedToken(next_token.clone()));
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    fn compile_atom(&mut self) -> Result<(), Error> {
        // Consume the current token and compile it.
        if let Some(token) = self.tokens.next() {
            match token.kind {
                TokenKind::Ident => {
                    // If this identifier is immediately followed by an equal sign, then we
                    // this becomes a store operation instead of a load operation.
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.kind == TokenKind::Equal {
                            let name = token.data.clone();
                            self.consume(TokenKind::Equal)?;

                            // Compile the left hand side of the assignment.
                            self.compile_member()?;

                            let id = self.runtime.get_global_index(&name) as u32;
                            self.code.push(Instruction::Store { index: id });

                            return Ok(());
                        }
                    }

                    let id = self.runtime.get_global_index(&token.data) as u32;
                    self.code.push(Instruction::Load { index: id });

                    if let Some(token) = self.tokens.peek() {
                        if token.kind == TokenKind::LParen {
                            //let sym = self.field_to_id_map.len() as u32;
                            //   let sym_name = token.data.clone();
                            //self.field_to_id_map.insert(sym_name, sym);

                            // Emit an `Invoke` instruction.
                            self.consume(TokenKind::LParen)?;

                            let mut args = 0u8;
                            while let Some(token) = self.tokens.peek() {
                                if token.kind == TokenKind::RParen {
                                    break;
                                } else {
                                    args += 1;
                                    self.compile_member()?;

                                    // Optional trailing comma.
                                    if let Some(token) = self.tokens.peek() {
                                        if token.kind == TokenKind::Comma {
                                            self.consume(TokenKind::Comma)?;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }

                            self.consume(TokenKind::RParen)?;

                            self.code.push(Instruction::Call { args });
                        }
                    }
                }
                TokenKind::String => {
                    let len = token.data.len();
                    let value = &token.data.clone()[1..len - 1];
                    let index = self.runtime.interner.intern(value.to_string());
                    self.code.push(Instruction::LoadString { index });
                }
                TokenKind::Number => {
                    let num = token.data.parse::<f64>().expect("bug: bad float");
                    let idx = self.constants.len();
                    debug_assert!(idx < u32::MAX as usize, "bug: too many constants");
                    self.constants.push(num);
                    self.code.push(Instruction::LoadConst { index: idx as u32 });
                }
                TokenKind::True => {
                    self.code.push(Instruction::LoadTrue);
                }
                TokenKind::False => {
                    self.code.push(Instruction::LoadFalse);
                }
                TokenKind::Nil => {
                    self.code.push(Instruction::LoadNil);
                }
                TokenKind::Alloc => {
                    self.code.push(Instruction::Alloc);
                }
                _ => return Err(Error::UnexpectedToken(token.clone())),
            }
        }

        Ok(())
    }
}
