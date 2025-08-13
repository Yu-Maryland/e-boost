
use egg::*;
use crate::utils::{language::*};
pub struct wight_size;
pub struct wight_depth;

use crate::Prop::*;


// impl CostFunction<Prop> for Mixcost{        
//             type Cost = usize;
//             fn cost<C>(&mut self, enode: &Prop, mut costs: C) -> Self::Cost
//             where
//                 C: FnMut(Id) -> Self::Cost,
//             {   
//                 //norm :[0.018601899062213562, 0.0, 0.3218090878220529, 1.0, 0.840425667722836]
//                 // 1.3996 ---  0.0186
//                 // 0.93312 --- 0
//                 // 9 ---  0.3218
//                 // 26 --- 1
//                 // 22 --- 0.840  



//                 let op = enode.to_string();
//                 let op_cost_area   = match op.as_str() {
//                     "!" => 322 ,
//                     "+" => 1000,
//                     "*"=> 840 ,
//                     //"&" => 0.0 as  f32,
//                     _=> 0 
//                 };
//                 let op_cost_delay   = match op.as_str() {
//                     "!" => 0 ,
//                     "+" => 19,
//                     "*"=> 19 ,
//                     //"&" => 0.0 as  f32,
//                     _=> 0
//                 };                
//                 //let alpha = generate_random_float();
//                 //let costsize = (enode.fold(1, |sum, id: Id| sum + f64::from(costs(id))));
//                 let costdepth = enode.fold(op_cost_delay, |sum, id: Id| sum + costs(id));
//                 //let costdepth = 1 + enode.fold(0, |max, id| max.max(costs(id) ));
//                 let costsize =enode.fold(op_cost_area, |sum, id| sum + costs(id));
//                 //println!("enode{},costdepth{}",enode.to_string(),costdepth);
//                 //let costdepth = op_cost + enode.fold(0, |max, id| max.max_random(costs(id) ));
//                 //let result = 1.2*costsize+(costdepth*5)as f64;
//                 let result =2*costsize+3*costdepth;
//                 //let result =costdepth;
//                 // println!("cost{}", result);
//                 //let result = alpha * costsize + (((1.0-alpha)*costdepth as f64) as f64);
//                 result
//             }
            
//         }


impl CostFunction<Prop> for wight_size{        
    type Cost = usize;
    fn cost<C>(&mut self, enode: &Prop, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {   
        let op = enode.to_string();
        let op_cost   = match op.as_str() {
            "!" => 1 ,
            "+" => 3,
            "*"=> 5,
            //"&" => 0.0 as  f32,
            _=> 1 
        };
        //let alpha = generate_random_float();
        //let costsize = (enode.fold(1, |sum, id: Id| sum + f64::from(costs(id))));
        //let costdepth = enode.fold(op_cost, |sum, id: Id| sum + costs(id));
        //let costdepth = 1 + enode.fold(0, |max, id| max.max(costs(id) ));
        let costsize =enode.fold(op_cost, |sum, id| sum + costs(id));
        //println!("enode{},costdepth{}",enode.to_string(),costdepth);
        //let costdepth = op_cost + enode.fold(0, |max, id| max.max_random(costs(id) ));
        //let result = 1.2*costsize+(costdepth*5)as f64;
        let result =costsize;
        //let result =costdepth;
        // println!("cost{}", result);
        //let result = alpha * costsize + (((1.0-alpha)*costdepth as f64) as f64);
        result as usize
    }
    
}

impl CostFunction<Prop> for wight_depth{        
    type Cost = usize;
    fn cost<C>(&mut self, enode: &Prop, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {   
        // let c = match enode {
        //     Not(a) => 1 + enode.fold(op_cost, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
        //     And([a,b]) => 2  + enode.fold(op_cost, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
        //     Or([a,b]) => 3  + enode.fold(op_cost, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
        //     _=>0  + enode.fold(op_cost, |max, id| max.max(costs(id))),  


        // };
        let c = match enode {
            Not(a) => 2 + enode.fold(0, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
            And([a,b]) => 4  + enode.fold(0, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
            Or([a,b]) =>  6 + enode.fold(0, |max, id| max.max(costs(id))),  // Change constants to floating point numbers
            _=>0  + enode.fold(0, |max, id| max.max(costs(id))),  


        };
        c
       
    }
    
}


pub struct AstSize;
impl<L: Language> CostFunction<L> for AstSize {
    type Cost = usize;
    fn cost<C>(&mut self, enode: &L, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        enode.fold(1, |sum, id: Id| sum.saturating_add(costs(id)))
    }
}

pub struct AstDepth;
impl<L: Language> CostFunction<L> for AstDepth {
    type Cost = usize;  // Change type to f64
    fn cost<C>(&mut self, enode: &L, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        1 + enode.fold(0, |max, id| max.max(costs(id)))  // Change constants to floating point numbers
    }
}