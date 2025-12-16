use std::{collections::HashMap, vec};

use crate::compiler::Module;

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    // Load a global variable.
    Load { index: u32 },
    // Store a global variable.
    Store { index: u32 },

    IndexGet { index: u32 },
    IndexSet { index: u32 },

    // Push `nil` to the top of the stack.
    LoadNil,
    // Push `true` to the top of the stack.
    LoadTrue,
    // Push `false` to the top of the stack.
    LoadFalse,
    // Push a constant unto the top of the stack.
    LoadConst { index: u32 },
    // Push a string to the top of the stack.
    LoadString { index: u32 },

    // Allocate a new object and push it to the top of the stack.
    Alloc,

    // Call a function.
    Call { args: u8 },
    // Invoke a method.
    Invoke { args: u8, sym: u32 },

    Jmp { addr: i32 },
    JmpIfTrue { addr: i32 },
    JmpIfFalse { addr: i32 },

    // Pop off the top of the stack.
    Pop,
    // Halt execution.
    Halt,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(u32),
    FunctionPtr(u32),
    Object(u32),
    Free(u32),
}

#[derive(Debug)]
pub struct Object {
    pub data: ahash::HashMap<u32, Value>,
}

impl Object {
    pub fn new() -> Self {
        Self {
            data: ahash::HashMap::default(),
        }
    }
}

pub struct FunctionArgs<'r> {
    pub stack: &'r mut Vec<Value>,
    pub heap: &'r mut Heap,
    pub strings: &'r mut Interner,
}

pub struct Runtime {
    globals: Vec<Value>,
    global_name_map: HashMap<String, usize>,
    pub field_to_id_map: ahash::HashMap<String, u32>,
    functions: Vec<FunctionDef>,
    stack: Vec<Value>,
    ip: usize,
    pub heap: Heap,
    pub interner: Interner,
}

#[derive(Default)]
pub struct Interner {
    pub strings: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, string: String) -> u32 {
        if let Some(idx) = self.strings.iter().position(|s| s == &string) {
            idx as u32
        } else {
            let idx = self.strings.len();
            self.strings.push(string);
            idx as u32
        }
    }

    pub fn get(&self, addr: u32) -> &String {
        debug_assert!(addr < self.strings.len() as _, "bug: invalid address");
        &self.strings[addr as usize]
    }
}

pub struct FunctionDef {
    func: Box<dyn Fn(FunctionArgs) -> Value>,
    args: u8,
}

impl Runtime {
    pub fn spawn_vm<'r>(&'r mut self, module: &'r Module) -> Vm<'r> {
        Vm { module, vm: self }
    }

    pub fn set_global(&mut self, name: impl ToString, value: Value) {
        let name = name.to_string();
        match self.global_name_map.get(&name) {
            Some(idx) => {
                self.globals[*idx] = value;
            }
            None => {
                let index = self.globals.len();
                self.globals.push(value);
                self.global_name_map.insert(name, index);
            }
        }
    }

    pub fn get_global_index(&mut self, name: &str) -> usize {
        match self.global_name_map.get(name) {
            Some(idx) => *idx,
            None => {
                let index = self.globals.len();
                self.globals.push(Value::Nil);
                self.global_name_map.insert(name.to_string(), index);
                index
            }
        }
    }

    pub fn register_function<F: Fn(FunctionArgs) -> Value + 'static>(
        &mut self,
        name: impl ToString,
        args: u8,
        f: F,
    ) {
        let index = self.functions.len() as u32;

        let def = FunctionDef {
            func: Box::new(f),
            args,
        };

        self.functions.push(def);
        self.set_global(name, Value::FunctionPtr(index));
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.ip = 0;
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            globals: vec![],
            global_name_map: Default::default(),
            field_to_id_map: Default::default(),
            interner: Default::default(),
            functions: vec![],
            stack: vec![],
            ip: 0,
            heap: Heap::new(20),
        }
    }
}

pub struct Heap {
    next_free: usize,
    objects: Vec<HeapValue>,
}

pub enum HeapValue {
    Free { next: usize },
    Object(Object),
}

impl Heap {
    pub fn new(size: usize) -> Self {
        let mut objects = vec![];

        for i in 0..size {
            objects.push(HeapValue::Free { next: i + 1 });
        }

        Self {
            next_free: 0,
            objects,
        }
    }

    /// Allocate a new object, returning it's "address" in the heap. This virtual
    /// address can be used to retrieve the object.
    pub fn alloc(&mut self) -> Option<u32> {
        if self.next_free < self.objects.len() {
            let index = self.next_free;

            match self.objects[self.next_free] {
                HeapValue::Object(_) => unreachable!("Cell is not free"),
                HeapValue::Free { next } => self.next_free = next,
            };

            let obj = Object::new();
            self.objects[index] = HeapValue::Object(obj);

            Some(index as u32)
        } else {
            None
        }
    }

    /// Returns `None` if the object has been freed.
    pub fn get(&self, index: u32) -> Option<&Object> {
        match &self.objects[index as usize] {
            HeapValue::Object(obj) => Some(obj),
            HeapValue::Free { .. } => None,
        }
    }

    /// Returns `None` if the object has been freed.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut Object> {
        match &mut self.objects[index as usize] {
            HeapValue::Object(obj) => Some(obj),
            HeapValue::Free { .. } => None,
        }
    }

    pub fn free(&mut self, addr: u32) {
        let addr = addr as usize;

        if addr >= self.objects.len() {
            panic!("bug: Invalid address");
        }

        let prev_free = self.next_free;
        self.objects[addr] = HeapValue::Free { next: prev_free };
        self.next_free = addr;
    }
}

pub enum ControlFlow {
    RequestGC,
    Continue,
    Halt,
}

pub struct Vm<'a> {
    pub vm: &'a mut Runtime,
    pub module: &'a Module,
}

impl<'a> Vm<'a> {
    pub fn step(&mut self) -> ControlFlow {
        let inst = self.module.code[self.vm.ip];
        self.vm.ip += 1;

        // println!("{:?}", inst);
        // println!("{:?}", self.vm.stack);

        match inst {
            Instruction::Load { index } => {
                let value = self.vm.globals[index as usize];
                self.vm.stack.push(value);
            }
            Instruction::Store { index } => {
                let new_value = self.vm.stack.pop().expect("bug: stack is empty");
                self.vm.globals[index as usize] = new_value;
            }
            Instruction::IndexGet { index } => {
                let value = self.vm.stack.pop().unwrap();
                if let Value::Object(addr) = value {
                    if let Some(obj) = self.vm.heap.get(addr) {
                        let field_value = obj.data.get(&index).copied().unwrap_or(Value::Nil);
                        self.vm.stack.push(field_value);
                    } else {
                        todo!("segfault (attempt to read freed object");
                    }
                } else {
                    todo!("not an object; need real errors");
                }
            }
            Instruction::IndexSet { index } => {
                let new_value = self.vm.stack.pop().unwrap();
                let value = self.vm.stack.pop().unwrap();
                if let Value::Object(addr) = value {
                    if let Some(obj) = self.vm.heap.get_mut(addr) {
                        obj.data.insert(index, new_value);
                    } else {
                        todo!("segfault (attempt to read freed object");
                    }
                } else {
                    todo!("not an object; need real errors");
                }
            }
            Instruction::LoadNil => {
                self.vm.stack.push(Value::Nil);
            }
            Instruction::LoadTrue => {
                self.vm.stack.push(Value::Bool(true));
            }
            Instruction::LoadFalse => {
                self.vm.stack.push(Value::Bool(false));
            }
            Instruction::LoadConst { index } => {
                let num = self.module.constants[index as usize];
                self.vm.stack.push(Value::Number(num));
            }
            Instruction::LoadString { index } => {
                self.vm.stack.push(Value::String(index)); // That's it. That's the whole joke.
            }
            Instruction::Alloc => {
                match self.vm.heap.alloc() {
                    Some(addr) => self.vm.stack.push(Value::Object(addr)),
                    None => {
                        // Repeat this instruction on the next step.
                        self.vm.ip -= 1;
                        return ControlFlow::RequestGC
                    }
                }
            }
            Instruction::Call { args } => {
                let func_offset = self.vm.stack.len() - (args as usize + 1);
                let func_ptr = self.vm.stack[func_offset];
                if let Value::FunctionPtr(ptr) = func_ptr {
                    let func_args = FunctionArgs {
                        stack: &mut self.vm.stack,
                        heap: &mut self.vm.heap,
                        strings: &mut self.vm.interner,
                    };
                    let def = &self.vm.functions[ptr as usize];

                    if def.args != args {
                        // TODO: don't panic.
                        if def.args > args {
                            panic!(
                                "missing arguments. Expected {} but only got {}",
                                def.args, args
                            );
                        } else {
                            panic!("Too many arguments. Expected {} but got {}", def.args, args);
                        }
                    }

                    let res = (def.func)(func_args);

                    self.vm.stack.truncate(func_offset);

                    self.vm.stack.push(res);
                } else {
                    todo!("expected function pointer");
                }
            }
            Instruction::Invoke { args, sym } => {
                // TODO: dispatch methods.
                // Hrm...
            }
            Instruction::Jmp { addr } => todo!(),
            Instruction::JmpIfTrue { addr } => todo!(),
            Instruction::JmpIfFalse { addr } => todo!(),
            Instruction::Pop => {
                self.vm.stack.pop();
            }
            Instruction::Halt => {
                return ControlFlow::Halt;
            }
        }

        // println!("-> {:?}\n", self.vm.stack);

        ControlFlow::Continue
    }
}
