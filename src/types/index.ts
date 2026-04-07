export interface Device {
  serial: string;
  status: string;
  model: string | null;
}

export interface SessionInfo {
  id: string;
  device_serial: string;
  app_package: string;
  display_spec: string;
  width: number;
  height: number;
  status: SessionStatus;
}

export interface CloneInfo {
  package: string;
  display_name: string;
  icon: string | null;
}

export interface GameAction {
  key: string;
  label: string;
  x: number;
  y: number;
  /** Zone width (normalized 0-1), tap is randomized within */
  w: number;
  /** Zone height (normalized 0-1) */
  h: number;
}

export interface NavigationShortcuts {
  new_session: string;
  close_session: string;
  next_tab: string;
  prev_tab: string;
}

export interface AppConfig {
  navigation: NavigationShortcuts;
  game_actions: GameAction[];
}

export type SessionStatus =
  | "Starting"
  | "Running"
  | "Stopped"
  | { Error: string };
