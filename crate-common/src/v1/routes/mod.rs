use utoipa::{
    openapi::{path::{HttpMethod, Operation}, Response},
    Path,
};

pub enum Method {
    Head,
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub trait Route {
    fn method() -> Method;
    fn path() -> &'static str;
    fn summary() -> &'static str;
    fn doc() -> &'static str;
    
    type Request;
    type Response;
}

#[cfg(feature = "utoipa")]
impl Into<HttpMethod> for Method {
    fn into(self) -> HttpMethod {
        match self {
            Method::Head => HttpMethod::Head,
            Method::Get => HttpMethod::Get,
            Method::Post => HttpMethod::Post,
            Method::Put => HttpMethod::Put,
            Method::Patch => HttpMethod::Patch,
            Method::Delete => HttpMethod::Delete,
        }
    }
}

pub struct Example;

impl Route for Example {
    fn method() -> Method {
        todo!()
    }

    fn path() -> &'static str {
        todo!()
    }

    fn summary() -> &'static str {
        todo!()
    }

    fn doc() -> &'static str {
        todo!()
    }

    type Request;

    type Response;
}

// impl Path for Example {
//     fn methods() -> Vec<HttpMethod> {
//         vec![HttpMethod::Get]
//     }

//     fn path() -> String {
//         "a".to_string()
//     }

//     fn operation() -> Operation {
//         let mut op = Operation::new();
//         op.responses.responses.insert("a", {
//             let res = Response::new("a");
//             res.description
//             res
//         })
//         // op.summary
//         // op.description
//         op
//     }
// }
