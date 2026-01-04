import { useState, useCallback } from 'react';

/**
 * A hook to manage state with undo/redo capability.
 * @param {any} initialState 
 * @returns [state, setState, undo, redo, canUndo, canRedo, resetState]
 */
export default function useHistory(initialState) {
    const [past, setPast] = useState([]);
    const [present, setPresent] = useState(initialState);
    const [future, setFuture] = useState([]);

    const canUndo = past.length > 0;
    const canRedo = future.length > 0;

    const undo = useCallback(() => {
        if (!canUndo) return;
        const previous = past[past.length - 1];
        const newPast = past.slice(0, past.length - 1);

        setFuture([present, ...future]);
        setPresent(previous);
        setPast(newPast);
    }, [past, present, future, canUndo]);

    const redo = useCallback(() => {
        if (!canRedo) return;
        const next = future[0];
        const newFuture = future.slice(1);

        setPast([...past, present]);
        setPresent(next);
        setFuture(newFuture);
    }, [past, present, future, canRedo]);

    const update = useCallback((newPresent, replace = false) => {
        if (replace) {
            setPresent(newPresent);
            return;
        }
        setPast([...past, present]);
        setPresent(newPresent);
        setFuture([]); // Clear future on new change
    }, [past, present]);

    const reset = useCallback((newState) => {
        setPast([]);
        setPresent(newState);
        setFuture([]);
    }, []);

    return [present, update, undo, redo, canUndo, canRedo, reset];
}
