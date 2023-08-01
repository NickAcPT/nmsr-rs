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

    // Allow to have multiple arguments of $name: $_type
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

            pub fn [<set_ $name>](&mut self, $name: $_type) {
                self.$inner.$name = $name;
                self.dirty = true;
            }
        }
    };
    ($inner: ident, $($name: ident),*) => {
        $(
            camera_inner_getters_setters!($inner, $name: f32);
        )*
    };
}

pub(crate) use camera_getters_setters;
pub(crate) use camera_inner_getters_setters;
