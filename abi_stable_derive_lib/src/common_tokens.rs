use proc_macro2::Span;

use syn::Ident;

#[derive(Debug)]
pub(crate) struct StartLenTokens{
    pub(crate) start_len:Ident,
    pub(crate) new:Ident,
}

impl StartLenTokens{
    pub(crate) fn new(span:Span)->Self{
        Self{
            start_len:Ident::new("__StartLen",span),
            new:Ident::new("new",span),
        }
    }
}


#[derive(Debug)]
pub(crate) struct FnPointerTokens{
    pub(crate) c_abi_lit: ::syn::LitStr,
    pub(crate) static_: Ident,
    pub(crate) underscore: Ident,
}

impl FnPointerTokens{
    pub fn new(span:Span)->Self{
        Self{
            c_abi_lit:syn::parse_str(r#""C""#).unwrap(),
            static_:Ident::new("static",span),
            underscore:Ident::new("_",span),
        }
    }
}



#[derive(Debug)]
pub(crate) struct LifetimeTokens{
    pub(crate) li_static: Ident,
    pub(crate) li_index: Ident,
}

impl LifetimeTokens{
    pub fn new(span:Span)->Self{
        Self{
            li_static:Ident::new("__LIStatic",span),
            li_index:Ident::new("__LIParam",span),
        }
    }
}


        