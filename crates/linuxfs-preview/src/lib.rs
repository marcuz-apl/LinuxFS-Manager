#[cfg(windows)]
pub const fn dark_caption_attribute() -> u32 {
    20
}

#[cfg(windows)]
pub fn centered_window_position(
    window_size: (i32, i32),
    work_area: (i32, i32, i32, i32),
) -> (i32, i32) {
    let (window_width, window_height) = window_size;
    let (left, top, work_width, work_height) = work_area;
    (
        left + (work_width - window_width).max(0) / 2,
        top + (work_height - window_height).max(0) / 2,
    )
}

#[cfg(windows)]
#[cfg(test)]
mod dark_caption_tests {
    use super::dark_caption_attribute;

    #[test]
    fn dark_caption_attribute_is_immersive_dark_mode() {
        assert_eq!(dark_caption_attribute(), 20);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::centered_window_position;

    #[test]
    fn centered_window_position_uses_work_area_origin_and_size() {
        assert_eq!(
            centered_window_position((1200, 820), (0, 0, 1680, 1010)),
            (240, 95)
        );
    }
}
