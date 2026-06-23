#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub connection: ConnectionState,
    pub printer: PrinterState,
    pub temperatures: TemperatureState,
    pub fans: FanState,
    pub hmi: HmiState,
    pub process: ProcessState,
    pub print: PrintState,
    pub files: FilesState,
}

// AppState специально хранит только модель приложения, без UART/WebSocket
// handle'ов. Это позволяет тестировать reducer и renderer без железа.
impl Default for AppState {
    fn default() -> Self {
        Self {
            connection: ConnectionState::default(),
            printer: PrinterState::default(),
            temperatures: TemperatureState::default(),
            fans: FanState::default(),
            hmi: HmiState::default(),
            process: ProcessState::default(),
            print: PrintState::default(),
            files: FilesState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionState {
    pub moonraker: ConnectionStatus,
    pub klipper: KlipperStatus,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            moonraker: ConnectionStatus::Disconnected,
            klipper: KlipperStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KlipperStatus {
    Unknown,
    Ready,
    Busy,
    Error,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterState {
    pub status: PrinterStatus,
    pub can_accept_commands: bool,
}

impl Default for PrinterState {
    fn default() -> Self {
        Self {
            status: PrinterStatus::Unknown,
            can_accept_commands: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterStatus {
    Unknown,
    Standby,
    Ready,
    Printing,
    Paused,
    Complete,
    Cancelled,
    Busy,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemperatureState {
    pub nozzle: HeaterState,
    pub bed: HeaterState,
    pub filament_load_target: f32,
}

impl Default for TemperatureState {
    fn default() -> Self {
        Self {
            nozzle: HeaterState::default(),
            bed: HeaterState::default(),
            filament_load_target: 230.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeaterState {
    pub current: f32,
    pub target: f32,
}

impl Default for HeaterState {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FanState {
    pub part: FanSpeed,
    pub side: FanSpeed,
    pub filter: FanSpeed,
}

impl Default for FanState {
    fn default() -> Self {
        Self {
            part: FanSpeed::default(),
            side: FanSpeed::default(),
            filter: FanSpeed::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FanSpeed {
    pub percent: u8,
}

impl Default for FanSpeed {
    fn default() -> Self {
        Self { percent: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HmiState {
    pub current_screen: Screen,
    pub requested_screen: Option<Screen>,
    pub navigation_locked: bool,
    pub global_touch_enabled: bool,
    pub move_distance: MoveDistance,
    // generation нужен для будущей отмены долгих задач вроде thumbnail worker:
    // пользователь ушел со страницы - старый job больше не должен дорисовывать.
    pub generation: u64,
}

impl Default for HmiState {
    fn default() -> Self {
        Self {
            current_screen: Page::Home,
            requested_screen: None,
            navigation_locked: false,
            global_touch_enabled: true,
            move_distance: MoveDistance::Mm10,
            generation: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Print,
    Printing,
    Files,
    Settings,
    MoveTemp,
    LoadUnload,
    Calibration,
    Network,
    Error,
    Unknown(u16),
}

pub type Screen = Page;

impl Page {
    pub fn id(self) -> u16 {
        match self {
            Self::Home => 0,
            Self::Print => 2,
            Self::Printing => 2,
            Self::MoveTemp => 3,
            Self::LoadUnload => 4,
            Self::Files => 54,
            Self::Settings => 11,
            Self::Calibration => 33,
            Self::Network => 18,
            Self::Error => 56,
            Self::Unknown(id) => id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDistance {
    Mm1,
    Mm10,
    Mm30,
}

impl MoveDistance {
    pub fn value_mm(self) -> u8 {
        match self {
            Self::Mm1 => 1,
            Self::Mm10 => 10,
            Self::Mm30 => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessState {
    pub active_operation: ActiveOperation,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            active_operation: ActiveOperation::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOperation {
    None,
    Homing,
    BedMesh,
    ZTilt,
    Shaper,
    LoadFilament,
    UnloadFilament,
    Printing,
    Stopping,
    Rebooting,
    Calibration,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_page(&mut self, page: Page) {
        self.hmi.current_screen = page;
        self.hmi.requested_screen = None;
        // Любая смена страницы инвалидирует фоновые UI-задачи.
        self.hmi.generation += 1;
    }

    pub fn request_page(&mut self, page: Page) -> bool {
        if self.hmi.navigation_locked || !self.hmi.global_touch_enabled {
            return false;
        }

        self.set_page(page);
        true
    }

    pub fn lock_navigation(&mut self, operation: ActiveOperation) {
        self.process.active_operation = operation;
        self.hmi.navigation_locked = true;
        self.hmi.global_touch_enabled = false;
    }

    pub fn unlock_navigation(&mut self) {
        self.process.active_operation = ActiveOperation::None;
        self.hmi.navigation_locked = false;
        self.hmi.global_touch_enabled = true;
    }

    pub fn set_nozzle_temperature(&mut self, current: f32, target: f32) {
        self.temperatures.nozzle.current = current;
        self.temperatures.nozzle.target = target;
    }

    pub fn set_bed_temperature(&mut self, current: f32, target: f32) {
        self.temperatures.bed.current = current;
        self.temperatures.bed.target = target;
    }

    pub fn set_fan_percent(&mut self, fan: FanKind, percent: u8) {
        let percent = percent.min(100);

        match fan {
            FanKind::Part => self.fans.part.percent = percent,
            FanKind::Side => self.fans.side.percent = percent,
            FanKind::Filter => self.fans.filter.percent = percent,
        }
    }

    pub fn can_send_printer_command(&self) -> bool {
        self.connection.moonraker == ConnectionStatus::Connected
            && self.connection.klipper == KlipperStatus::Ready
            && self.printer.can_accept_commands
            && self.process.active_operation == ActiveOperation::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanKind {
    Part,
    Side,
    Filter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintState {
    pub filename: Option<String>,
    pub progress_percent: u8,
    pub elapsed_seconds: u32,
    pub remaining_seconds: Option<u32>,
}

impl Default for PrintState {
    fn default() -> Self {
        Self {
            filename: None,
            progress_percent: 0,
            elapsed_seconds: 0,
            remaining_seconds: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilesState {
    pub visible_slots: [Option<FileSlot>; 3],
}

impl FilesState {
    pub fn visible_file_at(&self, slot: u8) -> Option<&FileSlot> {
        self.visible_slots.get(slot as usize)?.as_ref()
    }

    pub fn set_visible_slot(&mut self, slot: u8, file: Option<FileSlot>) {
        if let Some(target) = self.visible_slots.get_mut(slot as usize) {
            *target = file;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSlot {
    pub name: String,
    pub path: String,
}

impl FileSlot {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_safe() {
        let state = AppState::default();

        assert_eq!(state.connection.moonraker, ConnectionStatus::Disconnected);
        assert_eq!(state.connection.klipper, KlipperStatus::Unknown);
        assert_eq!(state.hmi.current_screen, Page::Home);
        assert_eq!(state.hmi.move_distance, MoveDistance::Mm10);
        assert!(!state.printer.can_accept_commands);
    }

    #[test]
    fn page_has_numeric_id() {
        assert_eq!(Page::Home.id(), 0);
        assert_eq!(Page::Print.id(), 2);
        assert_eq!(Page::Printing.id(), 2);
        assert_eq!(Page::Calibration.id(), 33);
        assert_eq!(Page::Unknown(77).id(), 77);
    }

    #[test]
    fn set_page_updates_page_and_generation() {
        let mut state = AppState::default();

        assert_eq!(state.hmi.generation, 0);

        state.set_page(Page::Settings);

        assert_eq!(state.hmi.current_screen, Page::Settings);
        assert_eq!(state.hmi.generation, 1);
    }

    #[test]
    fn request_page_is_blocked_when_navigation_locked() {
        let mut state = AppState::default();

        state.lock_navigation(ActiveOperation::Calibration);

        let accepted = state.request_page(Page::Settings);

        assert!(!accepted);
        assert_eq!(state.hmi.current_screen, Page::Home);
    }

    #[test]
    fn unlock_navigation_allows_page_change() {
        let mut state = AppState::default();

        state.lock_navigation(ActiveOperation::Calibration);
        state.unlock_navigation();

        let accepted = state.request_page(Page::Settings);

        assert!(accepted);
        assert_eq!(state.hmi.current_screen, Page::Settings);
    }

    #[test]
    fn temperatures_can_be_updated() {
        let mut state = AppState::default();

        state.set_nozzle_temperature(215.5, 220.0);
        state.set_bed_temperature(59.0, 60.0);

        assert_eq!(state.temperatures.nozzle.current, 215.5);
        assert_eq!(state.temperatures.nozzle.target, 220.0);
        assert_eq!(state.temperatures.bed.current, 59.0);
        assert_eq!(state.temperatures.bed.target, 60.0);
    }

    #[test]
    fn fan_percent_is_clamped_to_100() {
        let mut state = AppState::default();

        state.set_fan_percent(FanKind::Part, 150);

        assert_eq!(state.fans.part.percent, 100);
    }

    #[test]
    fn move_distance_has_value() {
        assert_eq!(MoveDistance::Mm1.value_mm(), 1);
        assert_eq!(MoveDistance::Mm10.value_mm(), 10);
        assert_eq!(MoveDistance::Mm30.value_mm(), 30);
    }

    #[test]
    fn can_send_printer_command_when_ready_and_idle() {
        let mut state = AppState::default();

        state.connection.moonraker = ConnectionStatus::Connected;
        state.connection.klipper = KlipperStatus::Ready;
        state.printer.can_accept_commands = true;

        assert!(state.can_send_printer_command());
    }

    #[test]
    fn cannot_send_printer_command_during_active_operation() {
        let mut state = AppState::default();

        state.connection.moonraker = ConnectionStatus::Connected;
        state.connection.klipper = KlipperStatus::Ready;
        state.printer.can_accept_commands = true;
        state.lock_navigation(ActiveOperation::Homing);

        assert!(!state.can_send_printer_command());
    }
}
