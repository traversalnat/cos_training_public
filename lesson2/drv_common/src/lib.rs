#![no_std]

pub struct Driver<'a> {
    pub name: &'a str,
    pub compatible: &'a str,
}

impl Driver<'_> {
    pub fn info<'a>(name: &'a str, compatible: &'a str) -> Driver<'a> {
        Driver {
            name,
            compatible,
        }
    }
}

type InitFn = fn() -> Driver<'static>;

pub struct CallEntry {
    pub init_fn: InitFn,
}

#[macro_export]
macro_rules! module {
    (type: $module_type:ident, name: $module_name:expr, compatible: $module_compatible:expr) => {
        #[used]
        #[link_section = ".init_calls"]
        static DRV0_ENTRY: CallEntry = CallEntry {
            init_fn: drv0_init_fn
        };

        fn drv0_init_fn() -> $module_type<'static> {
            $module_type::info($module_name, $module_compatible)
        }
    };
}
