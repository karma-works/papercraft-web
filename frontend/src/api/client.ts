import { Project, SettingsOptions, IslandId } from '../types';

const API_BASE = '/api';

export async function getStatus(): Promise<{ has_model: boolean }> {
    const response = await fetch(`${API_BASE}/status`);
    if (!response.ok) throw new Error('Failed to get status');
    return response.json();
}

export async function uploadModel(file: File): Promise<Project> {
    return uploadModelWithProgress(file, () => { });
}

export async function uploadModelWithProgress(
    file: File,
    onProgress: (percent: number) => void
): Promise<Project> {
    return new Promise((resolve, reject) => {
        const formData = new FormData();
        formData.append('file', file);

        const xhr = new XMLHttpRequest();
        xhr.open('POST', `${API_BASE}/upload`, true);

        xhr.upload.onprogress = (e) => {
            if (e.lengthComputable) {
                const percent = Math.round((e.loaded / e.total) * 100);
                onProgress(percent);
            }
        };

        xhr.onload = () => {
            if (xhr.status >= 200 && xhr.status < 300) {
                try {
                    resolve(JSON.parse(xhr.responseText));
                } catch (e) {
                    const error = new Error('Failed to parse response') as any;
                    error.responseText = xhr.responseText;
                    reject(error);
                }
            } else {
                const error = new Error(`Upload failed: ${xhr.status} ${xhr.statusText}`) as any;
                error.status = xhr.status;
                error.responseText = xhr.responseText;
                reject(error);
            }
        };

        xhr.onerror = () => {
            reject(new Error('Network error during upload'));
        };

        xhr.send(formData);
    });
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

    if (!response.ok) {
        const error = new Error('Failed to perform action') as any;
        error.response = response;
        throw error;
    }
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
    packIslands: (): Action => ({
        type: 'pack',
    }),
};
