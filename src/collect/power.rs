#[derive(Clone, Debug, Default)]
pub struct PowerSnapshot {
    /// System-wide power draw in watts
    pub system_watts: Option<f32>,
}
