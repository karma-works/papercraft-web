// API client for communicating with the papercraft backend

const API_BASE = '/api';

export async function getStatus() {
    const response = await fetch(`${API_BASE}/status`);
    if (!response.ok) throw new Error('Failed to get status');
    return response.json();
}

export async function uploadModel(file) {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${API_BASE}/upload`, {
        method: 'POST',
        body: formData,
    });

    if (!response.ok) throw new Error('Failed to upload model');
    return response.json();
}

export async function getProject() {
    const response = await fetch(`${API_BASE}/project`);
    if (!response.ok) throw new Error('Failed to get project');
    return response.json();
}

export async function performAction(action) {
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
    toggleFlap: (edge, action = 'Toggle') => ({
        type: 'toggleFlap',
        edge,
        action,
    }),

    cut: (edge, offset = null) => ({
        type: 'cut',
        edge,
        offset,
    }),

    join: (edge, priorityFace = null) => ({
        type: 'join',
        edge,
        priority_face: priorityFace,
    }),

    moveIsland: (island, delta) => ({
        type: 'moveIsland',
        island,
        delta,
    }),

    rotateIsland: (island, angle, center) => ({
        type: 'rotateIsland',
        island,
        angle,
        center,
    }),

    setOptions: (options, relocatePieces = false) => ({
        type: 'setOptions',
        options,
        relocate_pieces: relocatePieces,
    }),
};
