use tauri::{PhysicalPosition, PhysicalSize};

pub(super) fn placement(
    cursor: Option<PhysicalPosition<i32>>,
    area: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale: f64,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let pixels = |n: f64| (n * scale).round().max(1.0) as u32;
    let width = pixels(380.0).min(size.width);
    let height = pixels(200.0).min(size.height);
    let gap = i64::from(pixels(12.0));
    fn axis(anchor: Option<i32>, start: i32, space: u32, length: u32, gap: i64) -> i32 {
        let min = i64::from(start);
        let max = min + i64::from(space - length);
        let at = anchor.map(i64::from).map_or(max - gap, |point| {
            if point + gap <= max {
                point + gap
            } else {
                point - gap - i64::from(length)
            }
        });
        at.clamp(min, max).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
    (
        PhysicalPosition::new(
            axis(cursor.map(|p| p.x), area.x, size.width, width, gap),
            axis(cursor.map(|p| p.y), area.y, size.height, height, gap),
        ),
        PhysicalSize::new(width, height),
    )
}
