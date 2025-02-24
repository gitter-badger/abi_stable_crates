use arrayvec::ArrayVec;

use syn::visit_mut::VisitMut;
use syn::{Ident, TypePath,TraitItemType};

use quote::ToTokens;

use std::mem;

#[derive(Debug,Clone)]
pub enum ReplaceWith{
    Ident(Ident),
    Remove,
    Keep,
}

pub trait VisitMutWith{
    fn visit_mut_with<F>(&mut self,other:&mut SelfReplacer<F>)
    where
        F: FnMut(&Ident) -> Option<ReplaceWith>;
}

macro_rules! impl_visit_mut_with {
    ( 
        $( ($self_:ty,$method:path) ),*
        $(,)*
    ) => (
        $(
            impl VisitMutWith for $self_{
                #[inline]
                fn visit_mut_with<F>(&mut self,other:&mut SelfReplacer<F>)
                where
                    F: FnMut(&Ident) -> Option<ReplaceWith>,
                {
                    $method(other,self);
                }
            }
        )*
    )
}

impl_visit_mut_with!{
    (syn::WherePredicate,VisitMut::visit_where_predicate_mut),
    (TraitItemType,VisitMut::visit_trait_item_type_mut),
    (syn::Type,VisitMut::visit_type_mut),
}


pub(crate) fn replace_self_path<V,F>(
    value: &mut V,
    replace_with:ReplaceWith,
    is_assoc_type: F
) where
    V:VisitMutWith,
    F: FnMut(&Ident) -> Option<ReplaceWith>,
{
    value.visit_mut_with(&mut SelfReplacer { is_assoc_type , replace_with });
}

#[doc(hidden)]
pub struct SelfReplacer<F> {
    is_assoc_type: F,
    replace_with:ReplaceWith,
}

impl<F> VisitMut for SelfReplacer<F>
where
    F: FnMut(&Ident) -> Option<ReplaceWith>,
{
    fn visit_type_path_mut(&mut self, i: &mut TypePath) {
        if let Some(qself) = i.qself.as_mut() {
            self.visit_type_mut(&mut qself.ty);
        }
        
        
        let segments = &mut i.path.segments;

        for segment in &mut *segments {
            self.visit_path_arguments_mut(&mut segment.arguments);
        }
        
        // println!("\nbefore:{}",(&*segments).into_token_stream() );
        // println!("segments[1]:{}",segments.iter().nth(1).into_token_stream() );

        let is_self= segments[0].ident == "Self";

        match (segments.len(), is_self) {
            (0,true)|(1,true)=>panic!(
                "Self can't be used in a parameter,return type,or associated type.", 
            ),
            (2,true)=>{}
            (_,true)=>{
                panic!(
                    "Paths with 3 or more components are currently unsupported:\n{}\n", 
                    (&*segments).into_token_stream()
                );
            }
            (_,false)=>return,
        }

        let is_replaced=(self.is_assoc_type)(&segments[1].ident);
        // println!("is_replaced:{:?}",is_replaced );
        if let Some(replace_assoc_with)= is_replaced{
            let mut prev_segments = mem::replace(segments, Default::default()).into_iter();
            
            let replacements=[self.replace_with.clone(),replace_assoc_with];
            for replace_with in ArrayVec::from(replacements) {
                let prev_segment=prev_segments.next();
                match replace_with {
                    ReplaceWith::Ident(ident)=>{
                        segments.push(ident.into());
                    }
                    ReplaceWith::Remove=>{}
                    ReplaceWith::Keep=>{
                        segments.extend(prev_segment);
                    }
                }
            }

        }
        // println!("after:{}",(&*i).into_token_stream() );
    }
}
