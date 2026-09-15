mod construct;
mod indices;
mod invariants;
mod range;

use paramex_core::transfer::Transform;

fn transform_of(s: &str) -> Transform {
    match s {
        "sqrt" => Transform::Sqrt,
        "log" => Transform::Log,
        other => panic!("unknown transform {other:?}"),
    }
}
