use crate::stone::spec::Elem;

/// Discriminator n(x,y,z): z if x=y else x (StoneGral Def. 4.1).
pub fn discriminator(x: Elem, y: Elem, z: Elem) -> Elem {
    if x == y { z } else { x }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_basic() {
        assert_eq!(discriminator(0, 1, 2), 0);
        assert_eq!(discriminator(3, 3, 2), 2);
    }
}
