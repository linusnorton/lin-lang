use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use indexmap::IndexMap;

use crate::env::Env;
use lin_parse::ast::{Expr, Param};

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<String>),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<IndexMap<String, Value>>>),
    Function(Rc<Function>),
    Partial(Rc<PartialApp>),
    Iterator(Rc<RefCell<IteratorValue>>),
    NativeFunction(Rc<NativeFunction>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(v) => write!(f, "{:?}", v),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(a) => write!(f, "{:?}", a.borrow()),
            Value::Object(o) => write!(f, "{:?}", o.borrow()),
            Value::Function(_) => write!(f, "<function>"),
            Value::Partial(_) => write!(f, "<partial>"),
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::NativeFunction(nf) => write!(f, "<native:{}>", nf.name),
        }
    }
}

impl Value {
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(v) => {
                if *v == (*v as i64) as f64 && v.is_finite() {
                    format!("{:.1}", v)
                } else {
                    format!("{}", v)
                }
            }
            Value::String(s) => s.as_ref().clone(),
            Value::Array(a) => {
                let items: Vec<String> = a.borrow().iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Object(o) => {
                let fields: Vec<String> = o.borrow().iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_json_string()))
                    .collect();
                format!("{{{}}}", fields.join(", "))
            }
            Value::Function(_) => "<function>".to_string(),
            Value::Partial(_) => "<partial>".to_string(),
            Value::Iterator(_) => "<iterator>".to_string(),
            Value::NativeFunction(nf) => format!("<native:{}>", nf.name),
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            Value::String(s) => format!("\"{}\"", s),
            _ => self.to_display_string(),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn deep_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| x.deep_eq(y))
            }
            (Value::Object(a), Value::Object(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() {
                    return false;
                }
                a.iter().all(|(k, v)| b.get(k).map_or(false, |bv| v.deep_eq(bv)))
            }
            _ => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Boolean",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
            Value::Function(_) => "Function",
            Value::Partial(_) => "Function",
            Value::Iterator(_) => "Iterator",
            Value::NativeFunction(_) => "Function",
        }
    }
}

#[derive(Clone)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: Expr,
    pub closure: Env,
    pub arity: usize,
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function:{}>", self.name.as_deref().unwrap_or("anonymous"))
    }
}

#[derive(Clone, Debug)]
pub struct PartialApp {
    pub func: Value,
    pub applied: Vec<Value>,
}

pub struct IteratorValue {
    pub init: Value,
    pub cont: Value,
    pub next: Value,
    pub current: Value,
    pub state: Option<Value>,
    pub started: bool,
}

impl Clone for IteratorValue {
    fn clone(&self) -> Self {
        Self {
            init: self.init.clone(),
            cont: self.cont.clone(),
            next: self.next.clone(),
            current: self.current.clone(),
            state: self.state.clone(),
            started: self.started,
        }
    }
}

pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: usize,
    pub func: NativeFn,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native:{}>", self.name)
    }
}
