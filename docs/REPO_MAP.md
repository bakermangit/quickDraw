<repo_map>
### src/audio/mod.rs
```rust
pub struct AudioPlayer { config : AudioConfig , }
```

### src/config.rs
```rust
pub struct Config { pub general : GeneralConfig , pub trigger : TriggerConfig , pub audio : AudioConfig , pub logging : LoggingConfig , pub server : ServerConfig , }

pub struct GeneralConfig { pub mouse_input_method : String , pub keyboard_input_method : String , pub recognizer : String , pub confidence_threshold : f64 , pub gesture_profile : String , pub cursor_reset : bool , pub trace_overlay_enabled : bool , pub trace_color : String , pub trace_finesse_enabled : bool , pub trace_min_stroke : i32 , pub trace_max_stroke : i32 , pub trace_growth_rate : f64 , }

pub enum TriggerConfig { # [serde (alias = "button_combo" , rename = "combo")] Combo { # [serde (alias = "first")] key1 : String , # [serde (alias = "second")] key2 : String , } , # [serde (alias = "single_button" , alias = "keyboard_modifier" , rename = "single")] Single { # [serde (alias = "key" , alias = "button")] key1 : String , } , }

pub struct AudioConfig { pub enabled : bool , pub volume : f64 , pub success : String , pub error : String , }

pub struct LoggingConfig { pub level : String , }

pub struct ServerConfig { pub port : u16 , }

pub struct GestureProfile { pub gestures : Vec < GestureConfig > , }

pub struct GestureConfig { pub name : String , pub action : ActionConfig , pub sound : Option < String > , pub pattern : GesturePatternConfig , pub raw : GestureCapture , pub confidence_threshold : Option < f64 > , pub min_speed_px_per_ms : Option < f64 > , pub max_speed_px_per_ms : Option < f64 > , pub min_path_length_px : Option < f64 > , pub max_path_length_px : Option < f64 > , }

pub enum ActionConfig { KeyPress { key : VirtualKey , # [serde (default)] modifiers : Vec < VirtualKey > , } , }

pub struct GesturePatternConfig { pub algorithm : String , pub template_points : Vec < [f64 ; 2] > , # [serde (skip_serializing_if = "Option::is_none")] pub features : Option < Vec < f64 > > , }

pub fn get_config_dir () -> Result < PathBuf >;

pub fn load_config () -> Result < Config >;

pub fn load_gesture_profile (name : & str) -> Result < GestureProfile >;

pub fn save_gesture_profile (name : & str , profile : & GestureProfile) -> Result < () >;

pub fn parse_action_str (action_str : & str) -> Result < ActionConfig >;
```

### src/gesture/dollar_one.rs
```rust
pub struct DollarOneRecognizer { }
```

### src/gesture/mod.rs
```rust
pub trait GestureRecognizer : Send + 'static { # [doc = " Attempt to recognize a gesture from captured mouse data."] # [doc = " Returns the best match above the confidence threshold, or None."] fn recognize (& self , capture : & GestureCapture , templates : & [GestureTemplate] ,) -> Option < GestureMatch > ; # [doc = " Process a raw capture into a template for storage."] # [doc = " Called during gesture recording to generate the processed form."] fn create_template (& self , name : String , capture : & GestureCapture) -> GestureTemplate ; # [doc = " Human-readable name (e.g., \"dollar_one\", \"rubine\")"] # [allow (dead_code)] fn name (& self) -> & str ; }

pub trait GestureFilter : Send + 'static { # [doc = " Post-recognition filter. Returns true if the gesture should be accepted."] fn accept (& self , capture : & GestureCapture , template : & GestureTemplate) -> bool ; # [doc = " Human-readable name"] fn name (& self) -> & str ; }
```

### src/gesture/rubine.rs
```rust
/// Rubine Gesture Recognizer implementation.
///
/// This recognizer extracts 13 dynamic features from a gesture and compares them
/// to templates using a normalized Euclidean distance.
///
/// # Example
/// ```
/// use quickdraw::gesture::rubine::RubineRecognizer;
/// use quickdraw::gesture::GestureRecognizer;
/// use quickdraw::types::{GestureCapture, GestureTemplate};
///
/// let recognizer = RubineRecognizer::default();
/// let capture = GestureCapture {
///     points: vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
///     timestamps: vec![0, 10, 20],
/// };
/// let template = recognizer.create_template("test".to_string(), &capture);
/// let matches = recognizer.recognize(&capture, &[template]);
/// assert!(matches.is_some());
/// assert!(matches.unwrap().confidence > 0.9);
/// ```
pub struct RubineRecognizer ;
```

### src/input/hook.rs
```rust
pub struct HookInputSource { thread_handle : Option < JoinHandle < () > > , thread_id : Option < u32 > , }
```

### src/input/mod.rs
```rust
pub trait InputSource : Send + 'static { # [doc = " Start capturing input. Sends events through the provided channel."] # [doc = " This should spawn its own thread/task and return immediately."] fn start (& mut self , tx : Sender < InputEvent >) -> Result < () > ; # [doc = " Stop capturing input and clean up resources."] fn stop (& mut self) -> Result < () > ; # [doc = " Whether this input source can block/intercept events from reaching other apps."] # [doc = " Raw Input and polling are read-only (false). Hooks can intercept (true)."] # [allow (dead_code)] fn can_block (& self) -> bool ; # [doc = " Human-readable name for logging/config (e.g., \"raw_input\", \"hook\")"] # [allow (dead_code)] fn name (& self) -> & str ; }
```

### src/input/raw_input.rs
```rust
pub struct RawInputSource { thread_handle : Option < JoinHandle < () > > , # [cfg (windows)] window_handle : Option < SendHwnd > , # [cfg (not (windows))] window_handle : Option < () > , is_running : Arc < AtomicBool > , listen_mouse : bool , listen_keyboard : bool , }
```

### src/output/keyboard.rs
```rust
pub struct KeyPressAction { pub key : u16 , pub modifiers : Vec < u16 > , }

pub fn parse_virtual_key (name : & str) -> Result < u16 >;
```

### src/output/mod.rs
```rust
pub trait OutputAction : Send + 'static { # [doc = " Execute the action."] fn execute (& self) -> Result < () > ; # [doc = " Human-readable name for logging"] # [allow (dead_code)] fn name (& self) -> & str ; }

pub fn create_action (config : & ActionConfig) -> Result < Box < dyn OutputAction > >;
```

### src/pipeline.rs
```rust
pub enum TriggerState { Idle , WaitingForSecond { first : String } , GestureActive { origin : (f64 , f64) } , }

pub enum TriggerSignal { Pass (InputEvent) , GestureStarted , GesturePoint (f64 , f64) , GestureComplete , Nothing , }

pub struct CaptureRequest { pub result_tx : oneshot :: Sender < CaptureResult > , pub cancel_rx : oneshot :: Receiver < () > , }

pub struct CaptureResult { pub raw : GestureCapture , pub template : GestureTemplate , }

pub struct TriggerDetector { pub state : TriggerState , config : TriggerConfig , }

pub struct GestureAccumulator { capture_points : Vec < (f64 , f64) > , capture_timestamps : Vec < u64 > , current_x : f64 , current_y : f64 , start_time : Instant , origin_pos : (f64 , f64) , }

pub struct Pipeline { mouse_input_source : Box < dyn InputSource > , keyboard_input_source : Box < dyn InputSource > , recognizer : Box < dyn GestureRecognizer > , templates : Vec < GestureTemplate > , actions : HashMap < String , Box < dyn OutputAction > > , gesture_configs : HashMap < String , GestureConfig > , trigger : TriggerDetector , audio : AudioPlayer , config : Config , capture_request_rx : mpsc :: Receiver < CaptureRequest > , trace_overlay : Option < TraceOverlay > , }

pub fn build_pipeline (config : Config , capture_request_rx : mpsc :: Receiver < CaptureRequest >) -> Result < Pipeline >;
```

### src/server/handlers.rs
```rust
pub async fn handle_socket (socket : WebSocket , state : SharedState ,);
```

### src/server/mod.rs
```rust
pub struct ServerState { pub config : Config , pub gesture_profile : GestureProfile , pub capture_tx : mpsc :: Sender < CaptureRequest > , pub cmd_tx : mpsc :: Sender < SystemCommand > , }

pub type SharedState = Arc < Mutex < ServerState > > ;

pub async fn start (port : u16 , state : SharedState ,) -> anyhow :: Result < () >;
```

### src/tray/mod.rs
```rust
pub fn start_tray (cmd_tx : mpsc :: Sender < SystemCommand >) -> Result < () >;
```

### src/types.rs
```rust
/// A raw mouse input event from any InputSource.
pub struct InputEvent { pub event_type : InputEventType , # [doc = " Milliseconds, monotonic clock."] pub timestamp : u64 , }

/// Discriminated variants of an input event.
pub enum InputEventType { # [doc = " Relative mouse movement."] MouseMove { dx : i32 , dy : i32 } , # [doc = " Mouse button press or release."] MouseButton { button : MouseButton , pressed : bool } , # [doc = " Keyboard key press or release."] KeyboardKey { key : VirtualKey , pressed : bool } , }

/// Mouse buttons recognised by QuickDraw.
///
/// Derives `Serialize`/`Deserialize` because it appears in trigger config.
pub enum MouseButton { Left , Right , Middle , X1 , X2 , }

/// Accumulated mouse data captured during an active gesture recording.
///
/// Stored in gesture profile TOML (the `[gestures.raw]` section), so it
/// derives `Serialize`/`Deserialize`.
pub struct GestureCapture { # [doc = " Accumulated (x, y) positions relative to the gesture start point."] pub points : Vec < (f64 , f64) > , # [doc = " Milliseconds elapsed since the gesture started, one entry per point."] pub timestamps : Vec < u64 > , }

/// Result of a successful gesture recognition pass.
///
/// May be serialised when forwarded over the WebSocket IPC channel.
pub struct GestureMatch { # [doc = " Matches the `name` field of the winning `GestureTemplate`."] pub gesture_id : String , # [doc = " Normalised confidence score in the range 0.0 – 1.0."] pub confidence : f64 , }

/// A registered gesture loaded from a gesture-profile TOML file.
///
/// Fully serialisable so the config UI can round-trip templates over IPC.
pub struct GestureTemplate { # [doc = " Human-readable identifier, e.g. `\"flick-right\"`.  Must be unique within"] # [doc = " a profile."] pub name : String , # [doc = " Pre-processed template points produced by the recogniser's normalise"] # [doc = " step (resampled, scaled, rotated). For Rubine, this holds the raw points"] # [doc = " from the capture. Stored so the daemon can skip re-processing on every startup."] pub template_points : Vec < (f64 , f64) > , # [doc = " Statistical feature vector for recognizers that use it (e.g. Rubine)."] # [serde (skip_serializing_if = "Option::is_none")] pub features : Option < Vec < f64 > > , # [doc = " The algorithm that produced (and should match against) these points,"] # [doc = " e.g. `\"dollar_one\"`."] pub algorithm : String , }

/// A virtual key identifier as a human-readable string (e.g. `"F1"`, `"Ctrl"`).
///
/// Kept as a `String` newtype so config files stay readable without requiring
/// a large enum of every possible VK code.
pub struct VirtualKey (pub String) ;

/// An action to be executed by an `OutputAction` implementation.
///
/// Serialisable so actions can be round-tripped through the WebSocket IPC and
/// stored in gesture profile TOML.
pub enum ActionRequest { # [doc = " Simulate a key press, optionally with modifier keys held."] KeyPress { key : VirtualKey , # [serde (default)] modifiers : Vec < VirtualKey > , } , }

/// Commands sent to the main event loop to control the system.
pub enum SystemCommand { Quit , OpenConfig , ReloadEngine , }
```

### src/ui/trace.rs
```rust
pub enum TraceCommand { Begin (f64 , f64) , AddPoint (f64 , f64) , End , }

pub struct TraceOverlay { command_tx : mpsc :: Sender < TraceCommand > , hwnd : isize , }
```

</repo_map>
