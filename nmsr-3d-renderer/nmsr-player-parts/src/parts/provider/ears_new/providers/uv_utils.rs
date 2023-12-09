#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureRotation {
    None,
    Clockwise,
    CounterClockwise,
    UpsideDown,
}

impl TextureRotation {
    pub(crate) fn is_transposed(&self) -> bool {
        matches!(self, TextureRotation::Clockwise | TextureRotation::CounterClockwise)
    }
    
    pub(crate) fn cw(&self) -> TextureRotation {
        match self {
            TextureRotation::None => TextureRotation::Clockwise,
            TextureRotation::Clockwise => TextureRotation::UpsideDown,
            TextureRotation::UpsideDown => TextureRotation::CounterClockwise,
            TextureRotation::CounterClockwise => TextureRotation::None,
        }
    }
    
    pub(crate) fn ccw(&self) -> TextureRotation {
        match self {
            TextureRotation::None => TextureRotation::CounterClockwise,
            TextureRotation::CounterClockwise => TextureRotation::UpsideDown,
            TextureRotation::UpsideDown => TextureRotation::Clockwise,
            TextureRotation::Clockwise => TextureRotation::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureFlip {
    None,
    Horizontal,
    Vertical,
    Both,
}

impl TextureFlip {
    pub(crate) fn flip_horizontally(&self) -> TextureFlip {
        match self {
            TextureFlip::Both => TextureFlip::Vertical,
            TextureFlip::Horizontal => TextureFlip::None,
            TextureFlip::None => TextureFlip::Horizontal,
            TextureFlip::Vertical => TextureFlip::Both,
        }
    }
    
    pub(crate) fn flip_vertically(&self) -> TextureFlip {
        match self {
            TextureFlip::Both => TextureFlip::Horizontal,
            TextureFlip::Horizontal => TextureFlip::Both,
            TextureFlip::None => TextureFlip::Vertical,
            TextureFlip::Vertical => TextureFlip::None,
        }
    }
}