use std::cell::RefCell;
use cervus::engine;
use cervus::value_type::ValueType;

pub struct ControlFlow {
    blocks: Vec<ControlBlock>
}

impl ControlFlow {
    pub fn new() -> ControlFlow {
        ControlFlow {
            blocks: Vec::new()
        }
    }

    pub fn new_block(&mut self) -> &ControlBlock {
        self.blocks.push(ControlBlock::new());
        &self.blocks[self.blocks.len() - 1]
    }

    pub fn build<'a>(&self, m: &'a engine::Module, name: &str) -> engine::Function<'a> {
        let f = engine::Function::new(&m, name, ValueType::Int8, vec![ValueType::Int64]);
        let mut next_blk_id: usize = 0;
        for blk in &self.blocks {
            next_blk_id += 1;
            let bb = engine::BasicBlock::new(&f, format!("blk_{}", next_blk_id).as_str());
            let mut builder = engine::Builder::new(&bb);

            for op in blk.ops.borrow().iter() {
                op.build(&mut builder);
            }
        }
        f
    }
}

#[derive(Clone)]
pub struct ControlBlock {
    ops: RefCell<Vec<Op>>
}

impl ControlBlock {
    fn new() -> ControlBlock {
        ControlBlock {
            ops: RefCell::new(Vec::new())
        }
    }

    pub fn add_op(&self, op: Op) {
        self.ops.borrow_mut().push(op);
    }
}

#[derive(Clone)]
pub enum Op {
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value)
}

impl Op {
    pub fn build(&self, builder: &mut engine::Builder) {
        use cervus::engine::Action;
        match self {
            &Op::Add(ref a, ref b) => {
                if !a.check_type_class_eq(&b) {
                    panic!("check_type_class_eq failed");
                }
                if a.get_type_class() == TypeClass::Int {
                    builder.append(Action::IntAdd(a.to_value(), b.to_value()));
                } else if a.get_type_class() == TypeClass::Float {
                    builder.append(Action::FloatAdd(a.to_value(), b.to_value()));
                } else {
                    panic!("Unsupported type class");
                }
            },
            &Op::Sub(ref a, ref b) => {
                if !a.check_type_class_eq(&b) {
                    panic!("check_type_class_eq failed");
                }
                if a.get_type_class() == TypeClass::Int {
                    builder.append(Action::IntSub(a.to_value(), b.to_value()));
                } else if a.get_type_class() == TypeClass::Float {
                    builder.append(Action::FloatSub(a.to_value(), b.to_value()));
                } else {
                    panic!("Unsupported type class");
                }
            },
            &Op::Mul(ref a, ref b) => {
                if !a.check_type_class_eq(&b) {
                    panic!("check_type_class_eq failed");
                }
                if a.get_type_class() == TypeClass::Int {
                    builder.append(Action::IntMul(a.to_value(), b.to_value()));
                } else if a.get_type_class() == TypeClass::Float {
                    builder.append(Action::FloatMul(a.to_value(), b.to_value()));
                } else {
                    panic!("Unsupported type class");
                }
            },
            &Op::Div(ref a, ref b) => {
                if !a.check_type_class_eq(&b) {
                    panic!("check_type_class_eq failed");
                }
                if a.get_type_class() == TypeClass::Int {
                    builder.append(Action::SignedIntDiv(a.to_value(), b.to_value()));
                } else if a.get_type_class() == TypeClass::Float {
                    builder.append(Action::FloatDiv(a.to_value(), b.to_value()));
                } else {
                    panic!("Unsupported type class");
                }
            }
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum TypeClass {
    Int,
    Float
}

#[derive(Clone)]
pub enum Value {
    Constant(Constant),
    Variable(Variable)
}

impl Value {
    fn get_type_class(&self) -> TypeClass {
        match self {
            &Value::Constant(ref v) => v.get_type_class(),
            &Value::Variable(ref v) => v.get_type_class()
        }
    }

    pub fn to_value(&self) -> engine::Value {
        match self {
            &Value::Constant(ref v) => match v {
                &Constant::Int8(v) => engine::Value::from(v),
                &Constant::Int64(v) => engine::Value::from(v),
                &Constant::Float(v) => engine::Value::from(v),
            },
            &Value::Variable(ref v) => match v {
                &Variable::Int8(ref v) => v.clone(),
                &Variable::Int64(ref v) => v.clone(),
                &Variable::Float(ref v) => v.clone(),
            }
        }
    }

    fn check_type_class_eq(&self, other: &Value) -> bool {
        self.get_type_class() == other.get_type_class()
    }
}

#[derive(Clone)]
pub enum Constant {
    Int8(i8),
    Int64(i64),
    Float(f64)
}

impl Constant {
    fn get_type_class(&self) -> TypeClass {
        match self {
            &Constant::Int8(_) | &Constant::Int64(_) => TypeClass::Int,
            &Constant::Float(_) => TypeClass::Float
        }
    }
}

#[derive(Clone)]
pub enum Variable {
    Int8(engine::Value),
    Int64(engine::Value),
    Float(engine::Value)
}

impl Variable {
    fn get_type_class(&self) -> TypeClass {
        match self {
            &Variable::Int8(_) | &Variable::Int64(_) => TypeClass::Int,
            &Variable::Float(_) => TypeClass::Float
        }
    }
}
