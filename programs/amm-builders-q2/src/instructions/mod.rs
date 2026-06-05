pub mod initialize;
pub mod deposit;
pub mod withdraw;
pub mod swap;
pub mod lock;
pub mod burn;
pub mod payout;

pub use initialize::*;
pub use deposit::*;
pub use withdraw::*;
pub use swap::*;
pub use lock::*;
pub use burn::*;
pub use payout::*;

// TODO add update fees & locked & authority
// TODO implement fees collection to treasury

// TODO implement ur own CPMM lib (bonus)