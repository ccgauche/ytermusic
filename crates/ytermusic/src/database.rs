use database::YTLocalDatabase;
use once_cell::sync::Lazy;

use crate::consts::CACHE_DIR;

pub static DATABASE: Lazy<YTLocalDatabase> = Lazy::new(|| YTLocalDatabase::new(CACHE_DIR.clone()));
