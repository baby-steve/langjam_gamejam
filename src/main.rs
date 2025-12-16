use std::{env, io::Write};

use crate::{
    lexer::{Token, TokenKind},
    vm::{Runtime, Value},
};

mod compiler;
mod lexer;
mod vm;

#[derive(Debug)]
pub enum Error {
    UnexpectedCharacter(String),
    UnexpectedToken(Token),
    UnexpectedEOF,
    UnexpectedEOFExpected(TokenKind),
    UnexpectedTokenExpected(TokenKind, TokenKind),
}

fn main() -> Result<(), Error> {
    let mut runtime = Runtime::new();
    runtime.register_function("print", 1, |args| {
        let value = args.stack.pop().expect("missing arg");

        match value {
            Value::Nil => println!("nil"),
            Value::Bool(bool) => println!("{bool}"),
            Value::Number(num) => println!("{num}"),
            Value::String(idx) => {
                let string = args.strings.get(idx);
                println!("{string}");
            }
            Value::FunctionPtr(idx) => println!("fn<{idx}>"),
            Value::Object(idx) => match args.heap.get(idx) {
                Some(obj) => println!("{obj:?}"),
                None => println!("Object {{ <oops.__{idx}> }}"),
            },
            Value::Free(_) => todo!(),
        }

        Value::Nil
    });

    runtime.register_function("assert_eq", 3, |args| {
        let msg = args.stack.pop().unwrap();
        let expected = args.stack.pop().unwrap();
        let actual = args.stack.pop().unwrap();

        if expected != actual {
            panic!("Assertion failed: {:?}", msg);
        }

        Value::Nil
    });

    runtime.register_function("alloc", 0, |args| match args.heap.alloc() {
        Some(index) => Value::Object(index),
        None => {
            todo!("request that the user free up some memory");
        }
    });

    runtime.register_function("add", 2, |args| {
        let b = args.stack.pop().expect("missing arg 2");
        let a = args.stack.pop().expect("missing arg 1");
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::String(a), Value::String(b)) => {
                let str_a = args.strings.get(a);
                let str_b = args.strings.get(b);
                let mut new_str = str_a.clone();
                new_str.push_str(&str_b);
                let new_addr = args.strings.intern(new_str);
                Value::String(new_addr)
            }
            (Value::String(a), Value::Number(b)) => {
                let str_a = args.strings.get(a);
                let str_b = b.to_string();
                let mut new_str = str_a.clone();
                new_str.push_str(&str_b);
                let new_addr = args.strings.intern(new_str);
                Value::String(new_addr)
            }
            _ => panic!("invalid arguments"),
        }
    });

    macro_rules! bin_op_func {
        ($name:expr => $op:tt) => {
            runtime.register_function($name, 2, |args| {
                let b = args.stack.pop().expect("missing arg 2");
                let a = args.stack.pop().expect("missing arg 1");
                // println!("{a:?} : {b:?}");
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a $op b),
                    _ => panic!("invalid arguments"),
                }
            });
        };
    }

    bin_op_func!("sub" => -);
    bin_op_func!("div" => /);
    bin_op_func!("mul" => *);
    bin_op_func!("mod" => %);

    macro_rules! cmp_op_func {
        ($name:expr => $op:tt) => {
            runtime.register_function($name, 2, |args| {
                let b = args.stack.pop().expect("missing arg 2");
                let a = args.stack.pop().expect("missing arg 1");
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => Value::Bool(a $op b),
                    _ => panic!("valid arguments"),
                }
            });
        };
    }

    cmp_op_func!("gt" => >);
    cmp_op_func!("lt" => <);
    cmp_op_func!("lte" => <=);
    cmp_op_func!("gte" => >=);

    // Other math functions.
    macro_rules! simple_math {
        ($name:expr => $func:tt) => {
            runtime.register_function($name, 1, |args| match args.stack.pop().unwrap() {
                Value::Number(num) => Value::Number(num.$func()),
                _ => todo!(),
            });
        };
        ($name:expr => $func:tt => bool) => {
            runtime.register_function($name, 1, |args| match args.stack.pop().unwrap() {
                Value::Number(num) => Value::Bool(num.$func()),
                _ => todo!(),
            });
        };
    }

    // Math functions that return numbers.
    simple_math!("abs" => abs);
    simple_math!("acos" => acos);
    simple_math!("acosh" => acosh);
    simple_math!("asin" => asin);
    simple_math!("asinh" => asinh);
    simple_math!("atan" => atan);
    simple_math!("atanh" => atanh);
    simple_math!("cbrt" => cbrt);
    simple_math!("ceil" => ceil);
    simple_math!("cos" => cos);
    simple_math!("cosh" => cosh);
    simple_math!("exp" => exp);
    simple_math!("exp2" => exp2);
    simple_math!("floor" => floor);
    simple_math!("fract" => fract);
    simple_math!("ln" => ln);
    simple_math!("log10" => log10);
    simple_math!("log2" => log2);
    simple_math!("round" => round);
    simple_math!("signum" => signum);
    simple_math!("sin" => sin);
    simple_math!("sinh" => sinh);
    simple_math!("sqrt" => sqrt);
    simple_math!("tan" => tan);
    simple_math!("tanh" => tanh);
    simple_math!("to_degrees" => to_degrees);
    simple_math!("to_radians" => to_radians);
    simple_math!("trunc" => trunc);
    // Math functions that return true or false.
    simple_math!("is_finite" => is_finite => bool);
    simple_math!("is_infinite" => is_infinite => bool);
    simple_math!("is_nan" => is_nan => bool);
    simple_math!("is_normal" => is_normal => bool);

    runtime.register_function("len", 1, |args| {
        match args.stack.pop().unwrap() {
            Value::String(addr) => {
                let str = args.strings.get(addr);
                Value::Number(str.len() as f64)
            }
            Value::Object(addr) => {
                let obj = args.heap.get(addr);
                if let Some(obj) = obj {
                    Value::Number(obj.data.len() as f64)
                } else {
                    todo!("real errors. This is a segfault");
                }
            }
            v => panic!("Expected string or object, got {:?}", v),
        }
    });

    let args: Vec<String> = env::args().collect();
    if let Some(path) = args.get(1) {
        let src = std::fs::read_to_string(path).expect("error reading file");

        run(src, &mut runtime)?;
    } else {
        println!("♥ Welcome to Nuclear Alabaster Chainsaw - v0.0.1 ♥");
        println!("(Type ':exit' to quit)\n");
        print!("> ");
        std::io::stdout().flush().unwrap();

        let stdin = std::io::stdin();

        for line in stdin.lines() {
            let line = line.unwrap();
            if line == ":exit" {
                // Exit
                println!("bye!");
                break;
            } else {
                run(line, &mut runtime)?;
            }

            print!("> ");
            std::io::stdout().flush().unwrap();
        }
    }

    Ok(())
}

fn run(src: String, runtime: &mut Runtime) -> Result<(), Error> {
    let tokens = lexer::lex(&src)?;

    // for token in tokens.iter() {
    //     println!("{token:?}");
    // }

    let module = compiler::compile(tokens, runtime)?;
    // println!("{module:#?}");

    let mut vm = runtime.spawn_vm(&module);

    loop {
        match vm.step() {
            vm::ControlFlow::Continue => continue,
            vm::ControlFlow::Halt => break,
            vm::ControlFlow::RequestGC => {
                println!("Garbage collection triggered");

                // For now, randomly remove an object.
                vm.vm.heap.free(0);
                vm.vm.heap.free(1);
                vm.vm.heap.free(2);
                vm.vm.heap.free(3);
                vm.vm.heap.free(4);
            }
        }
    }

    runtime.reset();

    Ok(())
}
