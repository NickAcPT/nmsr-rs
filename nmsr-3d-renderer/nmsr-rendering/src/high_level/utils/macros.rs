
macro_rules! camera_getters_setters {
    ($name: ident: $_type: ty) => {
        paste::paste! {
            pub fn [<get_ $name>](&self) -> $_type {
                self.$name
            }

            pub fn [<get_ $name _as_mut>](&mut self) -> &mut $_type {
                &mut self.$name
            }

            pub fn [<set_ $name>](&mut self, $name: $_type) {
                self.$name = $name;
                self.dirty = true;
            }
        }
    };
    ($($name: ident: $_type: ty),*) => {
        $(
            camera_getters_setters!($name: $_type);
        )*
    };
}

macro_rules! camera_inner_getters_setters {
    ($inner: ident, $name: ident: $_type: ty) => {
        paste::paste! {
            pub fn [<get_ $name>](&self) -> $_type {
                self.$inner.$name
            }

            pub fn [<get_ $name _as_mut>](&mut self) -> &mut $_type {
                &mut self.$inner.$name
            }

            pub fn [<set_ $name>](&mut self, $name: $_type) {
                self.$inner.$name = $name;
                self.dirty = true;
            }
        }
    };($inner: ident(), $name: ident: $_type: ty, $prefix: ident) => {
        paste::paste! {
            pub fn [<get_ $prefix _ $name>](&self) -> $_type {
                self.$inner().$name
            }

            pub fn [<set_ $prefix _ $name>](&mut self, $name: $_type) {
                if let Some([<$prefix _ $name>]) = self.[<$inner _as_mut>]() {
                    [<$prefix _ $name>].$name = $name;
                }
                self.dirty = true;
            }
        }
    };
    ($inner: ident, $($name: ident),*) => {
        $(
            camera_inner_getters_setters!($inner, $name: f32);
        )*
    };
    ($inner: ident(), $prefix: ident, $($name: ident),*) => {
        $(
            camera_inner_getters_setters!($inner(), $name: f32, $prefix);
        )*
    };
}

macro_rules! camera_inner_getters_setters_opt {
    ($inner: ident, $name: ident: $_type: ty, $default: expr) => {
        paste::paste! {
            pub fn [<get_ $name>](&self) -> $_type {
                self.$inner.[<get_ $name>]().unwrap_or($default)
            }

            pub fn [<get_ $name _as_mut>](&mut self) -> Option<&mut $_type> {
                self.$inner.[<as_mut_ $name>]()
            }

            pub fn [<set_ $name>](&mut self, $name: $_type) {
                if let Some([<$name _mut>]) = self.$inner.[<as_mut_ $name>]() {
                    *[<$name _mut>] = $name;
                }
                self.dirty = true;
            }
        }
    };
    ($inner: ident, $($name: ident),*) => {
        $(
            camera_inner_getters_setters_opt!($inner, $name: f32, 0f32);
        )*
    };
}

pub(crate) use camera_getters_setters;
pub(crate) use camera_inner_getters_setters;
pub(crate) use camera_inner_getters_setters_opt;