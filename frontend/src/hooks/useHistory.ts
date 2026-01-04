import { useState, useCallback } from 'react';

/**
 * A hook to manage state with undo/redo capability.
 * @param {T} initialState 
 * @returns [state, setState, undo, redo, canUndo, canRedo, resetState]
 */
export default function useHistory<T>(initialState: T): [
    T,
    (newState: T, replace?: boolean) => void,
    () => void,
    () => void,
    boolean,
    boolean,
    (newState: T) => void
] {
    const [past, setPast] = useState<T[]>([]);
    const [present, setPresent] = useState<T>(initialState);
    const [future, setFuture] = useState<T[]>([]);

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

    const update = useCallback((newPresent: T, replace = false) => {
        if (replace) {
            setPresent(newPresent);
            return;
        }
        setPast([...past, present]);
        setPresent(newPresent);
        setFuture([]); // Clear future on new change
    }, [past, present]);

    const reset = useCallback((newState: T) => {
        setPast([]);
        setPresent(newState);
        setFuture([]);
    }, []);

    return [present, update, undo, redo, canUndo, canRedo, reset];
}
