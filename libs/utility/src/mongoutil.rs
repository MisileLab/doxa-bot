use mongodb::bson;
use bson::Document;
use serde::{Serialize};

pub fn bson2docs<T>(x:&T) -> Document 
    where T: Sized + Serialize {
    let bson = bson::to_bson(x).unwrap();
    let docs = bson::from_bson::<Document>(bson).unwrap();
    return docs;
}