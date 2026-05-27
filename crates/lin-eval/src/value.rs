use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::sync::{Arc, Mutex};
use indexmap::IndexMap;

use crate::env::Env;
use lin_parse::ast::{Expr, Param};

/// State of a Promise<T>.
pub enum PromiseState {
    Pending,
    Resolved(JsonValue),
    Failed(String),
}

/// JSON-compatible value that can be safely sent across thread boundaries.
/// Lin's spec requires async thunk return types to be JSON-compatible.
#[derive(Clone, Debug)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
    Error(String),
}

impl JsonValue {
    pub fn to_value(&self) -> Value {
        match self {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Bool(*b),
            JsonValue::Int(i) => Value::Int(*i),
            JsonValue::Float(f) => Value::Float(*f),
            JsonValue::String(s) => Value::String(Rc::new(s.clone())),
            JsonValue::Array(items) => {
                let vals: Vec<Value> = items.iter().map(|v| v.to_value()).collect();
                Value::Array(Rc::new(RefCell::new(vals)))
            }
            JsonValue::Object(fields) => {
                let mut map = IndexMap::new();
                for (k, v) in fields {
                    map.insert(k.clone(), v.to_value());
                }
                Value::Object(Rc::new(RefCell::new(map)))
            }
            JsonValue::Error(msg) => {
                let mut map = IndexMap::new();
                map.insert("type".to_string(), Value::String(Rc::new("error".to_string())));
                map.insert("message".to_string(), Value::String(Rc::new(msg.clone())));
                Value::Object(Rc::new(RefCell::new(map)))
            }
        }
    }
}

impl Value {
    /// Convert to a JSON-safe value for thread boundaries. Returns Err if non-serializable.
    pub fn to_json_value(&self) -> Result<JsonValue, String> {
        match self {
            Value::Null => Ok(JsonValue::Null),
            Value::Bool(b) => Ok(JsonValue::Bool(*b)),
            Value::Int(i) => Ok(JsonValue::Int(*i)),
            Value::Float(f) => Ok(JsonValue::Float(*f)),
            Value::String(s) => Ok(JsonValue::String(s.as_ref().clone())),
            Value::Array(a) => {
                let items: Result<Vec<JsonValue>, String> = a.borrow().iter()
                    .map(|v| v.to_json_value())
                    .collect();
                Ok(JsonValue::Array(items?))
            }
            Value::Object(o) => {
                let fields: Result<Vec<(String, JsonValue)>, String> = o.borrow().iter()
                    .map(|(k, v)| v.to_json_value().map(|jv| (k.clone(), jv)))
                    .collect();
                Ok(JsonValue::Object(fields?))
            }
            Value::Function(_) | Value::Partial(_) | Value::NativeFunction(_) => {
                Err("Functions cannot be passed across thread boundaries".to_string())
            }
            Value::Iterator(_) => {
                Err("Iterators cannot be passed across thread boundaries".to_string())
            }
            Value::Promise(_) | Value::ThreadPool(_) | Value::Worker(_) => {
                Err("Concurrency primitives cannot be passed across thread boundaries".to_string())
            }
        }
    }
}

/// A thread pool — opaque wrapper around a fixed-size rayon-style manual pool.
pub struct ThreadPoolState {
    pub sender: std::sync::mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

pub struct WorkerState {
    pub sender: std::sync::mpsc::SyncSender<WorkerMsg>,
    pub closed: std::sync::atomic::AtomicBool,
}

pub enum WorkerMsg {
    Message(JsonValue, Option<std::sync::mpsc::SyncSender<JsonValue>>),
    Shutdown,
}

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
    /// A future value produced by `async`.
    Promise(Arc<Mutex<PromiseState>>),
    /// A fixed-size thread pool produced by `threadPool(n)`.
    ThreadPool(Arc<ThreadPoolState>),
    /// A stateful worker thread produced by `worker(onMsg, onShutdown)`.
    Worker(Arc<WorkerState>),
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
            Value::Promise(_) => write!(f, "<promise>"),
            Value::ThreadPool(_) => write!(f, "<threadpool>"),
            Value::Worker(_) => write!(f, "<worker>"),
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
            Value::Promise(_) => "<promise>".to_string(),
            Value::ThreadPool(_) => "<threadpool>".to_string(),
            Value::Worker(_) => "<worker>".to_string(),
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
            Value::Promise(_) => "Promise",
            Value::ThreadPool(_) => "ThreadPool",
            Value::Worker(_) => "Worker",
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

/// A wrapper for a deep-cloned `Value` that is safe to send across thread boundaries.
/// SAFETY: The value must have been deep-cloned so that no `Rc` is shared with
/// any other thread. Lin's spec guarantees this by forbidding `var` capture in
/// async thunks — all captured values are immutable and deep-copied at lambda creation.
pub struct SendValue(pub Value);
// SAFETY: Lin guarantees the inner Value has no shared Rc references
// when created by deep-cloning an immutable val capture.
unsafe impl Send for SendValue {}

/// A deep-cloned `Function` wrapped in a raw pointer to bypass Rc's !Send.
/// SAFETY: Function closures passed to async must not capture var bindings.
/// All captures are deep-cloned (Rc refcount = 1, no shared aliasing across threads).
pub struct SendFunction(pub *mut Function);
unsafe impl Send for SendFunction {}

impl SendFunction {
    pub fn new(f: Rc<Function>) -> Self {
        // Leak the Rc to get a raw pointer we own. We'll reconstruct the Rc in the thread.
        SendFunction(Rc::into_raw(f) as *mut Function)
    }

    /// Reconstruct the Rc<Function> from the raw pointer. Only call once.
    ///
    /// SAFETY: Caller must ensure this is called exactly once and no other Rc
    /// references to the same allocation exist.
    pub unsafe fn into_rc(self) -> Rc<Function> {
        Rc::from_raw(self.0 as *const Function)
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
