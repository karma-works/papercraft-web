import { Project, SettingsOptions, IslandId } from '../types';

const API_BASE = '/api';

export async function getStatus(): Promise<{ has_model: boolean }> {
    const response = await fetch(`${API_BASE}/status`);
    if (!response.ok) throw new Error('Failed to get status');
    return response.json();
}

export async function uploadModel(file: File): Promise<Project> {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${API_BASE}/upload`, {
        method: 'POST',
        body: formData,
    });

    if (!response.ok) throw new Error('Failed to upload model');
    return response.json();
}

export async function getProject(): Promise<Project> {
    const response = await fetch(`${API_BASE}/project`);
    if (!response.ok) throw new Error('Failed to get project');
    return response.json();
}

interface Action {
    type: string;
    [key: string]: any;
}

export async function performAction(action: Action): Promise<Project> {
    const response = await fetch(`${API_BASE}/action`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(action),
    });

    if (!response.ok) throw new Error('Failed to perform action');
    return response.json();
}

// Action helpers
export const actions = {
    toggleFlap: (edge: any, action = 'Toggle'): Action => ({
        type: 'toggleFlap',
        edge,
        action,
    }),

    cut: (edge: any, offset: number | null = null): Action => ({
        type: 'cut',
        edge,
        offset,
    }),

    join: (edge: any, priorityFace: number | null = null): Action => ({
        type: 'join',
        edge,
        priority_face: priorityFace,
    }),

    moveIsland: (island: IslandId, delta: [number, number]): Action => ({
        type: 'moveIsland',
        island,
        delta,
    }),

    rotateIsland: (island: IslandId, angle: number, center: [number, number]): Action => ({
        type: 'rotateIsland',
        island,
        angle,
        center,
    }),

    setOptions: (options: SettingsOptions, relocatePieces = false): Action => ({
        type: 'setOptions',
        options,
        relocate_pieces: relocatePieces,
    }),
};
