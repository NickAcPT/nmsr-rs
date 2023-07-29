macro_rules! camera_getters_setters {
    ($name: ident: $_type: ty) => {
        paste::paste! {
            pub fn [<get_ $name>](&self) -> &$_type {
                &self.$name
            }

            pub fn [<set_ $name>](&mut self, $name: $_type) {
                self.$name = $name;
                self.dirty = true;
            }
        }
    };

    // Allow to have multiple arguments of $name: $_type
    ($($name: ident: $_type: ty),*) => {
        $(
            camera_getters_setters!($name: $_type);
        )*
    };
}

pub(crate) use camera_getters_setters;
