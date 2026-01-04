export interface Vertex {
    p: number[];
    n: number[];
    t: number[];
}

export type PointOrArray = { x: number; y: number } | [number, number];

export interface Face {
    m: number;
    vs: number[];
    es: number[];
    vertices?: PointOrArray[]; // Used in 2D canvas
}

export interface Edge {
    id: any; // Used as key in maps or for hover. Could be string or number.
    start: PointOrArray;
    end: PointOrArray;
    kind: string;
}

// 3D model edge (different from 2D papercraft edge)
export interface ModelEdge {
    f0: number;
    f1: number | null;
    v0: number;
    v1: number;
}

export interface Flap {
    vertices: PointOrArray[];
}

export interface IslandId {
    idx: number;
    version?: number;
}

export interface Island {
    id: IslandId;
    pos: PointOrArray;
    edges?: Edge[];
    faces?: Face[];
    flaps?: Flap[];
}

export interface ModelData {
    vs: Vertex[];
    fs: Face[];
    es: ModelEdge[];
}

export interface SettingsOptions {
    page_size: [number, number];
    margin: [number, number, number, number];
    texture: boolean;
    tab_width?: number;
    tab_angle?: number;
    [key: string]: any;
}

export interface Project {
    model: ModelData | null;
    islands?: Island[];
    options?: SettingsOptions;
}

export interface Status {
    connected: boolean;
    has_model: boolean; // Note: API returns snake_case 'has_model'
}
