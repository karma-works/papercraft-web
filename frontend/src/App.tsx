import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Button, FileTrigger, ToggleButton } from 'react-aria-components';
import {
  Upload, Scissors, Link2, Move, RotateCw, Settings,
  ZoomIn, ZoomOut, Maximize2, Box, Origami,
  MousePointer2, Hand, Undo2, Redo2, HelpCircle
} from 'lucide-react';
import * as api from './api/client';
import Preview3D from './Preview3D';
import SettingsDialog from './SettingsDialog';
import useHistory from './hooks/useHistory';
import { Project, IslandId, SettingsOptions, PointOrArray } from './types';

// Constants
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 5;
const ZOOM_STEP = 0.1;

// Status Indicator Component
function StatusIndicator({ connected, hasModel }: { connected: boolean; hasModel: boolean }) {
  return (
    <div className="status-indicator">
      <span className={`status-dot ${connected ? 'connected' : 'disconnected'}`} />
      <span>{connected ? (hasModel ? 'Model loaded' : 'Ready') : 'Disconnected'}</span>
    </div>
  );
}

// File Upload Component
function FileUpload({ onUpload, isLoading, compact = false }: { onUpload: (file: File) => void; isLoading: boolean; compact?: boolean }) {
  const [isDragOver, setIsDragOver] = useState(false);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    const file = e.dataTransfer?.files[0];
    if (file) onUpload(file);
  }, [onUpload]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  // Compact version for header/toolbar
  if (compact) {
    return (
      <FileTrigger
        acceptedFileTypes={['.obj', '.stl', '.pdo', '.pbo']}
        onSelect={(files) => {
          const file = files?.[0];
          if (file) onUpload(file);
        }}
      >
        <Button className="btn btn-secondary" style={{ padding: '0.4rem 0.75rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          {isLoading ? <div className="spinner" style={{ width: 16, height: 16 }} /> : <Upload size={16} />}
          <span>Load Model</span>
        </Button>
      </FileTrigger>
    );
  }

  return (
    <div
      className={`file-upload ${isDragOver ? 'drag-over' : ''}`}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <div className="file-upload-icon">
        {isLoading ? <div className="spinner" /> : <Upload size={48} />}
      </div>
      <div className="file-upload-text">
        <strong>Drop a 3D model file here</strong>
        <br />
        or click to browse (OBJ, STL, PDO)
      </div>
      <FileTrigger
        acceptedFileTypes={['.obj', '.stl', '.pdo', '.pbo']}
        onSelect={(files) => {
          const file = files?.[0];
          if (file) onUpload(file);
        }}
      >
        <Button className="btn btn-primary" style={{ marginTop: '1rem' }}>
          Browse Files
        </Button>
      </FileTrigger>
    </div>
  );
}

// Zoom Controls Component
interface ZoomControlsProps {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomFit: () => void;
}

function ZoomControls({ zoom, onZoomIn, onZoomOut, onZoomFit }: ZoomControlsProps) {
  return (
    <div className="zoom-controls">
      <button className="zoom-btn" onClick={onZoomOut} title="Zoom Out" data-testid="zoom-out-btn">
        <ZoomOut size={16} />
      </button>
      <span className="zoom-level">{Math.round(zoom * 100)}%</span>
      <button className="zoom-btn" onClick={onZoomIn} title="Zoom In" data-testid="zoom-in-btn">
        <ZoomIn size={16} />
      </button>
      <button className="zoom-btn" onClick={onZoomFit} title="Fit to View">
        <Maximize2 size={16} />
      </button>
    </div>
  );
}

// Toolbar Component
interface ViewOptions {
  showFlaps: boolean;
  showTextures: boolean;
  [key: string]: boolean;
}

interface ToolbarProps {
  mode: string;
  onModeChange: (mode: string) => void;
  viewOptions: ViewOptions;
  onViewOptionChange: (key: string, value: boolean) => void;
  onOpenSettings: () => void;
  onExport: (format: string) => void;
  onUndo: () => void;
  onRedo: () => void;
  canUndo: boolean;
  canRedo: boolean;
}

// Toolbar Component
function Toolbar({ mode, onModeChange, viewOptions, onViewOptionChange, onOpenSettings, onExport, onUndo, onRedo, canUndo, canRedo }: ToolbarProps) {
  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={onUndo}
          disabled={!canUndo}
          title="Undo (Ctrl+Z)"
          data-testid="undo-btn"
        >
          <Undo2 size={18} style={{ opacity: canUndo ? 1 : 0.3 }} />
        </button>
        <button
          className="toolbar-btn"
          onClick={onRedo}
          disabled={!canRedo}
          title="Redo (Ctrl+Shift+Z)"
          data-testid="redo-btn"
        >
          <Redo2 size={18} style={{ opacity: canRedo ? 1 : 0.3 }} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'select' ? 'active' : ''}`}
          onClick={() => onModeChange('select')}
          title="Select (V)"
          data-testid="mode-select-btn"
        >
          <MousePointer2 size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'pan' ? 'active' : ''}`}
          onClick={() => onModeChange('pan')}
          title="Pan (H)"
          data-testid="mode-pan-btn"
        >
          <Hand size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'move' ? 'active' : ''}`}
          onClick={() => onModeChange('move')}
          title="Move Islands (M)"
          data-testid="mode-move-btn"
        >
          <Move size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'rotate' ? 'active' : ''}`}
          onClick={() => onModeChange('rotate')}
          title="Rotate Islands (R)"
          data-testid="mode-rotate-btn"
        >
          <RotateCw size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'cut' ? 'active' : ''}`}
          onClick={() => onModeChange('cut')}
          title="Cut Edge (C)"
          data-testid="mode-cut-btn"
        >
          <Scissors size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'join' ? 'active' : ''}`}
          onClick={() => onModeChange('join')}
          title="Join Edges (J)"
          data-testid="mode-join-btn"
        >
          <Link2 size={18} />
        </button>
      </div>
      <div className="toolbar-group" title="Toggle Flaps (F)">
        <ToggleButton
          className={`toolbar-btn ${viewOptions.showFlaps ? 'active' : ''}`}
          isSelected={viewOptions.showFlaps}
          onChange={() => onViewOptionChange('showFlaps', !viewOptions.showFlaps)}
          aria-label="Toggle Flaps"
          data-testid="toggle-flaps-btn"
        >
          <span style={{ fontSize: '12px', fontWeight: 'bold' }}>F</span>
        </ToggleButton>
      </div>
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={() => onExport('pdf')}
          title="Export PDF"
        >
          <span style={{ fontSize: '10px', fontWeight: 'bold' }}>PDF</span>
        </button>
        <button
          className="toolbar-btn"
          onClick={() => onExport('svg')}
          title="Export SVG"
        >
          <span style={{ fontSize: '10px', fontWeight: 'bold' }}>SVG</span>
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={onOpenSettings}
          title="Settings"
        >
          <Settings size={18} />
        </button>
      </div>
    </div>
  );
}

// Helper to normalize point
function getPoint(p: PointOrArray): { x: number, y: number } {
  if (Array.isArray(p)) return { x: p[0], y: p[1] };
  return p;
}

// Interactive 2D Canvas Component
interface Canvas2DProps {
  project: Project | null;
  mode: string;
  viewOptions: ViewOptions;
  selectedIslands: number[];
  onSelectIsland: (id: IslandId | null, addToSelection?: boolean) => void;
  onMoveIsland: (id: IslandId, delta: [number, number]) => Promise<void>;
  onRotateIsland: (id: IslandId, angle: number, center: [number, number]) => Promise<void>;
  onProjectUpdate: (project: Project) => void;
  onError: (msg: string) => void;
  hoveredEdge: any | null;
  onHoverEdge: (edge: any | null) => void;
}

function Canvas2D({ project, mode, viewOptions, selectedIslands, onSelectIsland, onMoveIsland, onRotateIsland, onProjectUpdate, onError, hoveredEdge, onHoverEdge }: Canvas2DProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(2);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState<{ x: number, y: number } | null>(null);

  interface DraggedIslandState {
    id: IslandId;
    mode: 'move' | 'rotate';
    startX?: number;
    startY?: number;
    origX: number;
    origY: number;
    currentX?: number;
    currentY?: number;
    deltaX?: number;
    deltaY?: number;
    startAngle?: number;
    deltaAngle?: number;
  }

  const [draggedIsland, setDraggedIsland] = useState<DraggedIslandState | null>(null);
  const [hoveredIsland, setHoveredIsland] = useState<number | null>(null);
  // hoveredEdge state is now lifted to parent App component

  // Pointer tracking for multi-touch
  const pointersRef = useRef<Map<number, { x: number, y: number }>>(new Map());
  const prevPinchRef = useRef<{
    dist: number;
    center: { x: number, y: number };
    startZoom: number;
    startPan: { x: number, y: number };
  } | null>(null);

  // Get island data
  const islands = useMemo(() => {
    if (!project?.islands) return [];
    return Object.values(project.islands).map(data => ({ ...data }));
  }, [project]);

  // Calculate canvas dimensions
  const options = project?.options;
  const scale = 2; // pixels per mm base scale
  const pageWidth = options ? options.page_size[0] * scale : 210 * scale;
  const pageHeight = options ? options.page_size[1] * scale : 297 * scale;

  // Hit test - find island at position
  // Helper for point-segment distance (squared)
  const distToSegmentSquared = (p: { x: number, y: number }, v: { x: number, y: number }, w: { x: number, y: number }) => {
    const l2 = (v.x - w.x) ** 2 + (v.y - w.y) ** 2;
    if (l2 === 0) return (p.x - v.x) ** 2 + (p.y - v.y) ** 2;
    let t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2;
    t = Math.max(0, Math.min(1, t));
    return (p.x - (v.x + t * (w.x - v.x))) ** 2 + (p.y - (v.y + t * (w.y - v.y))) ** 2;
  };

  const hitTestEdge = useCallback((x: number, y: number): { islandId: IslandId; edgeId: any; edge: any; island: any } | null => {
    if (!project || !project.islands) return null;
    let hit: { islandId: IslandId; edgeId: any; edge: any; island: any } | null = null;
    let minD2 = Infinity;
    const threshold = 2; // mm
    const thresholdSq = threshold * threshold;

    project.islands.forEach(island => {
      if (island.edges) {
        island.edges.forEach(edge => {
          if (!edge.start || !edge.end) return;

          const s = getPoint(edge.start);
          const e = getPoint(edge.end);

          const d2 = distToSegmentSquared({ x, y }, s, e);
          if (d2 < thresholdSq && d2 < minD2) {
            minD2 = d2;
            hit = { islandId: island.id, edgeId: edge.id, edge, island };
          }
        });
      }
    });

    return hit;
  }, [project]);

  const hitTest = useCallback((clientX: number, clientY: number) => {
    const canvas = canvasRef.current;
    if (!canvas || !project) return null;

    const rect = canvas.getBoundingClientRect();
    const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x;
    const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;

    // Convert screen coordinates to global paper coordinates (pixels)
    const paperX = (clientX - offsetX) / zoom;
    const paperY = (clientY - offsetY) / zoom;

    // Scale back to mm for checking against island.pos which is in mm?
    // Wait, island.pos is in mm.
    // So convert paper coordinates to mm.
    const modelX = paperX / scale;
    const modelY = paperY / scale;

    // Find island at this position
    for (const island of islands) {
      const { x: ix, y: iy } = getPoint(island.pos);

      const dx = modelX - ix;
      const dy = modelY - iy;
      const distance = Math.sqrt(dx * dx + dy * dy);

      // Hit radius of 20mm
      if (distance < 20) {
        return island;
      }
    }
  }, [project, islands, zoom, pan, pageWidth, pageHeight, scale]);

  const performEdgeAction = useCallback(async (edge: any, altKey: boolean) => {
    console.log("Performing edge action:", edge, "Mode:", mode, "Alt:", altKey);
    try {
      let actionData;

      // Alt+Click for Flap Toggle
      if (altKey) {
        actionData = api.actions.toggleFlap(edge.id);
      } else if (mode === 'join' || (mode === 'select' && edge.kind === 'cut')) {
        actionData = api.actions.join(edge.id);
      } else if (mode === 'cut' || (mode === 'select' && (edge.kind === 'mountain' || edge.kind === 'valley'))) {
        actionData = api.actions.cut(edge.id, 5.0);
      }

      if (actionData) {
        console.log("Sending action to backend:", actionData);
        const updatedProject = await api.performAction(actionData);
        onProjectUpdate(updatedProject);
        onHoverEdge(null);
      } else {
        console.log("No action determined for this mode/edge combination.");
      }
    } catch (err: any) {
      console.error("Action failed:", err);
      onError(err.message);
    }
  }, [mode, onProjectUpdate, onError]);

  // Handle pointer down
  const handlePointerDown = useCallback(async (e: React.PointerEvent) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    pointersRef.current.set(e.pointerId, { x: e.clientX, y: e.clientY });

    // Multi-touch pinch start
    if (pointersRef.current.size === 2) {
      const points = Array.from(pointersRef.current.values());
      const dist = Math.hypot(points[0].x - points[1].x, points[0].y - points[1].y);
      const center = {
        x: (points[0].x + points[1].x) / 2,
        y: (points[0].y + points[1].y) / 2
      };

      prevPinchRef.current = { dist, center, startZoom: zoom, startPan: { ...pan } };

      // Cancel any single-finger drag
      setDraggedIsland(null);
      setIsDragging(false);
      setDragStart(null);
      return;
    }

    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (mode === 'pan' || e.button === 1) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
      // e.currentTarget.setPointerCapture(e.pointerId); // Already captured above
      return;
    }

    // Check edge click first
    if (hoveredEdge) {
      e.stopPropagation();
      await performEdgeAction(hoveredEdge, e.altKey);
      // If we are in cut/join mode, don't fallback to island selection
      if (mode === 'cut' || mode === 'join') return;
    }

    const island = hitTest(x, y);

    if (mode === 'select' || mode === 'move' || mode === 'rotate') {
      if (island) {
        onSelectIsland(island.id, e.shiftKey);

        const { x: ix, y: iy } = getPoint(island.pos);
        const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x; // Recalculate offsetX/Y
        const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;

        if (mode === 'move') {
          setDraggedIsland({
            id: island.id,
            mode: 'move',
            startX: x,
            startY: y,
            origX: ix,
            origY: iy
          });
          setIsDragging(true);
          // e.currentTarget.setPointerCapture(e.pointerId); // Already captured above
        } else if (mode === 'rotate') {
          // Calculate center in screen pixels
          const cx = ix * scale * zoom + offsetX;
          const cy = iy * scale * zoom + offsetY;
          const angle = Math.atan2(y - cy, x - cx);

          setDraggedIsland({
            id: island.id,
            mode: 'rotate',
            startAngle: angle,
            deltaAngle: 0,
            origX: ix,
            origY: iy
          });
          setIsDragging(true);
          // e.currentTarget.setPointerCapture(e.pointerId); // Already captured above
        }
      } else if (!e.shiftKey) {
        onSelectIsland(null);
      }
    }
  }, [mode, pan, hitTest, onSelectIsland, zoom, scale, pageWidth, pageHeight, hoveredEdge, performEdgeAction, onProjectUpdate, onError]);

  // Handle pointer move
  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    pointersRef.current.set(e.pointerId, { x: e.clientX, y: e.clientY });

    // Multi-touch pinch
    if (pointersRef.current.size === 2 && prevPinchRef.current) {
      const points = Array.from(pointersRef.current.values());
      const newDist = Math.hypot(points[0].x - points[1].x, points[0].y - points[1].y);
      const newCenter = {
        x: (points[0].x + points[1].x) / 2,
        y: (points[0].y + points[1].y) / 2
      };

      const pinch = prevPinchRef.current;
      const scaleFactor = newDist / pinch.dist;
      let newZoom = pinch.startZoom * scaleFactor;
      newZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, newZoom));

      // Calculate pan adjustment to keep the pinch center fixed
      const zoomRatio = newZoom / pinch.startZoom;
      const panX = pinch.startPan.x + (pinch.center.x - newCenter.x) + (newCenter.x - pinch.center.x * zoomRatio);
      const panY = pinch.startPan.y + (pinch.center.y - newCenter.y) + (newCenter.y - pinch.center.y * zoomRatio);

      setZoom(newZoom);
      setPan({ x: panX, y: panY });
      return;
    }

    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (isDragging) {
      if (draggedIsland) {
        e.currentTarget.setPointerCapture(e.pointerId); // Ensure capture is maintained
        if (draggedIsland.mode === 'move') {
          // Moving an island
          const startX = draggedIsland.startX!;
          const startY = draggedIsland.startY!;
          const dx = (x - startX) / zoom / scale;
          const dy = (y - startY) / zoom / scale;

          setDraggedIsland(prev => {
            if (!prev) return null;
            return {
              ...prev,
              currentX: prev.origX + dx,
              currentY: prev.origY + dy,
              deltaX: dx,
              deltaY: dy
            };
          });
        } else if (draggedIsland.mode === 'rotate') {
          const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x;
          const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;
          const cx = draggedIsland.origX * scale * zoom + offsetX;
          const cy = draggedIsland.origY * scale * zoom + offsetY;

          const angle = Math.atan2(y - cy, x - cx);
          const deltaAngle = angle - draggedIsland.startAngle!;

          setDraggedIsland(prev => {
            if (!prev) return null;
            return {
              ...prev,
              deltaAngle: deltaAngle
            };
          });
        }
      } else if (dragStart) {
        // Panning
        setPan({
          x: e.clientX - dragStart.x,
          y: e.clientY - dragStart.y
        });
      }
    } else {
      // Hover detection
      const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x;
      const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;

      // Convert screen coordinates to model coordinates (mm) for hitTestEdge
      const modelX = (x - offsetX) / (zoom * scale);
      const modelY = (y - offsetY) / (zoom * scale);

      const edgeHit = hitTestEdge(modelX, modelY);
      if (edgeHit) {
        onHoverEdge(edgeHit.edge); // Use edge object or ID depending on type. edgeHit has edgeId and edge.
        setHoveredIsland(null); // Clear island hover to avoid confusion
      } else {
        onHoverEdge(null);
        const island = hitTest(x, y); // Use screen coordinates for island hitTest
        setHoveredIsland(island ? island.id.idx : null);
      }
    }
  }, [isDragging, draggedIsland, dragStart, zoom, scale, hitTest, hitTestEdge, pan, pageWidth, pageHeight]);

  // Handle pointer up
  const handlePointerUp = useCallback(async (e: React.PointerEvent) => {
    pointersRef.current.delete(e.pointerId);
    e.currentTarget.releasePointerCapture(e.pointerId);

    if (pointersRef.current.size < 2) {
      prevPinchRef.current = null;
    }

    if (draggedIsland) {
      if (draggedIsland.mode === 'move' && draggedIsland.deltaX !== undefined) {
        if ((Math.abs(draggedIsland.deltaX || 0) > 0.1 || Math.abs(draggedIsland.deltaY || 0) > 0.1) && draggedIsland.deltaX !== undefined && draggedIsland.deltaY !== undefined) {
          await onMoveIsland(draggedIsland.id, [draggedIsland.deltaX, draggedIsland.deltaY]);
        }
      } else if (draggedIsland.mode === 'rotate' && draggedIsland.deltaAngle !== undefined) {
        if (Math.abs(draggedIsland.deltaAngle) > 0.01) {
          await onRotateIsland(draggedIsland.id, draggedIsland.deltaAngle, [draggedIsland.origX, draggedIsland.origY]);
        }
      }
    }

    setIsDragging(false);
    setDragStart(null);
    setDraggedIsland(null);
  }, [draggedIsland, onMoveIsland, onRotateIsland]);

  // Handle wheel zoom
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
    setZoom(z => Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z + delta)));
  }, []);

  // Zoom controls
  const handleZoomIn = useCallback(() => {
    setZoom(z => Math.min(MAX_ZOOM, z + ZOOM_STEP));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoom(z => Math.max(MIN_ZOOM, z - ZOOM_STEP));
  }, []);

  const handleZoomFit = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const scaleX = (rect.width - 40) / pageWidth;
    const scaleY = (rect.height - 40) / pageHeight;
    setZoom(Math.min(scaleX, scaleY, 1));
    setPan({ x: 0, y: 0 });
  }, [pageWidth, pageHeight]);

  // Draw canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !project) return;

    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * window.devicePixelRatio;
    canvas.height = rect.height * window.devicePixelRatio;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(window.devicePixelRatio, window.devicePixelRatio);

    // Clear canvas
    ctx.fillStyle = '#1e1e2e';
    ctx.fillRect(0, 0, rect.width, rect.height);

    // Calculate offsets
    const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x;
    const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;

    // Draw grid
    ctx.save();
    ctx.strokeStyle = '#2a2a3e';
    ctx.lineWidth = 0.5;
    const gridSize = 20 * zoom;
    for (let x = offsetX % gridSize; x < rect.width; x += gridSize) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, rect.height);
      ctx.stroke();
    }
    for (let y = offsetY % gridSize; y < rect.height; y += gridSize) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(rect.width, y);
      ctx.stroke();
    }
    ctx.restore();

    // Draw paper background
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(offsetX, offsetY, pageWidth * zoom, pageHeight * zoom);
    ctx.strokeStyle = '#4a4a5e';
    ctx.lineWidth = 1;
    ctx.strokeRect(offsetX, offsetY, pageWidth * zoom, pageHeight * zoom);

    // Clip to paper
    ctx.save();
    ctx.beginPath();
    ctx.rect(offsetX, offsetY, pageWidth * zoom, pageHeight * zoom);
    ctx.clip();

    // Draw islands
    islands.forEach((island) => {
      // Use idx for comparison
      const isSelected = selectedIslands.includes(island.id.idx);
      const isHovered = hoveredIsland === island.id.idx;
      const isDragged = draggedIsland?.id.idx === island.id.idx;

      // Extract pos and rot
      const { x: ix, y: iy } = getPoint(island.pos);

      let tx = 0;
      let ty = 0;
      let rot = 0;

      if (isDragged) {
        if (draggedIsland.mode === 'move') {
          tx = draggedIsland.deltaX || 0;
          ty = draggedIsland.deltaY || 0;
        } else if (draggedIsland.mode === 'rotate') {
          rot = draggedIsland.deltaAngle || 0;
        }
      }

      ctx.save();

      // Base transform to convert mm to canvas pixels (and paper offset)
      ctx.translate(offsetX, offsetY);
      ctx.scale(zoom * scale, zoom * scale);

      // Apply drag translation (in mm)
      if (tx !== 0 || ty !== 0) {
        ctx.translate(tx, ty);
      }

      // Apply rotation
      // Rotation needs to be around island center (ix, iy)
      // Visual rotation: translate to center, rotate, translate back
      if (rot !== 0) {
        ctx.translate(ix, iy);
        ctx.rotate(rot);
        ctx.translate(-ix, -iy);
      }

      // Draw Flaps (behind faces)
      if (viewOptions.showFlaps && island.flaps) {
        island.flaps.forEach(flap => {
          if (!flap.vertices || flap.vertices.length < 3) return;
          ctx.beginPath();
          flap.vertices.forEach((v, i) => {
            const { x: vx, y: vy } = getPoint(v);
            if (i === 0) ctx.moveTo(vx, vy);
            else ctx.lineTo(vx, vy);
          });
          ctx.closePath();
          ctx.fillStyle = '#e5e7eb'; // light grey
          ctx.fill();
          ctx.strokeStyle = '#9ca3af'; // darker grey
          ctx.lineWidth = 0.5 / scale;
          ctx.stroke();
        });
      }

      // Draw Faces
      if (island.faces) {
        island.faces.forEach(face => {
          if (!face.vertices || face.vertices.length < 3) return;

          ctx.beginPath();
          face.vertices.forEach((v, i) => {
            const { x: vx, y: vy } = getPoint(v);
            if (i === 0) ctx.moveTo(vx, vy);
            else ctx.lineTo(vx, vy);
          });
          ctx.closePath();

          // Style
          ctx.fillStyle = isSelected ? '#e0e7ff' : (isHovered ? '#f3f4f6' : '#ffffff');
          ctx.fill();
          ctx.strokeStyle = '#000000';
          ctx.lineWidth = 0.5 / scale; // constant pixel width approx
          ctx.stroke();
        });
      }

      // Draw Edges
      if (island.edges) {
        island.edges.forEach(edge => {
          const s = getPoint(edge.start);
          const e = getPoint(edge.end);
          const sx = s.x;
          const sy = s.y;
          const ex = e.x;
          const ey = e.y;

          const isHoveredEdge = hoveredEdge && hoveredEdge.id === edge.id;

          ctx.beginPath();
          ctx.moveTo(sx, sy);
          ctx.lineTo(ex, ey);

          if (isHoveredEdge) {
            ctx.lineWidth = 3 / scale;
            ctx.strokeStyle = '#f59e0b'; // Amber-500
            ctx.setLineDash([]);
          } else {
            ctx.lineWidth = 1 / scale;
            if (edge.kind === 'cut') {
              ctx.strokeStyle = '#000000';
              ctx.lineWidth = 1.5 / scale;
              ctx.setLineDash([]);
            } else if (edge.kind === 'mountain') {
              ctx.strokeStyle = '#ef4444'; // Red
              ctx.setLineDash([4 / scale, 2 / scale, 1 / scale, 2 / scale]);
            } else if (edge.kind === 'valley') {
              ctx.strokeStyle = '#3b82f6'; // Blue
              ctx.setLineDash([4 / scale, 4 / scale]);
            } else {
              ctx.strokeStyle = 'rgba(0,0,0,0.1)';
              ctx.setLineDash([]);
            }
          }
          ctx.stroke();
          ctx.setLineDash([]); // Reset
        });
      }

      // Draw Island Marker (Center)
      const markerSize = (isSelected ? 6 : 4) / scale;
      ctx.beginPath();
      ctx.arc(ix, iy, markerSize, 0, Math.PI * 2);
      ctx.fillStyle = isSelected ? '#6366f1' : (isHovered ? '#a78bfa' : 'rgba(99, 102, 241, 0.5)');
      ctx.fill();

      // Rotation Handle
      if (isSelected && mode === 'rotate') {
        const handleDist = 15; // mm
        // We need to rotate the handle position visually too if we are rotating!
        // But we already applied context rotation above for the whole island drawing group.
        // So drawing at (ix + 15, iy) will be rotated correctly by the context.
        const handleX = ix + handleDist;
        const handleY = iy;

        ctx.beginPath();
        ctx.moveTo(ix, iy);
        ctx.lineTo(handleX, handleY);
        ctx.strokeStyle = '#6366f1';
        ctx.lineWidth = 1 / scale;
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(handleX, handleY, 3 / scale, 0, Math.PI * 2);
        ctx.fillStyle = '#6366f1';
        ctx.fill();
      }

      ctx.restore();
    });

    ctx.restore(); // Clip restore

  }, [project, islands, zoom, pan, selectedIslands, hoveredIsland, draggedIsland, mode, viewOptions, pageWidth, pageHeight, scale]);

  // Keyboard shortcuts (local to Canvas2D for edge interaction)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      if (e.key === 'Enter' || e.key === ' ') {
        if (hoveredEdge) {
          e.preventDefault();
          performEdgeAction(hoveredEdge, e.altKey);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [hoveredEdge, performEdgeAction]);

  // Keyboard shortcuts (global to Canvas2D)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;

      switch (e.key.toLowerCase()) {
        case '+':
        case '=':
          handleZoomIn();
          break;
        case '-':
          handleZoomOut();
          break;
        case '0':
          handleZoomFit();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleZoomIn, handleZoomOut, handleZoomFit]);

  return (
    <div ref={containerRef} className="canvas-container">
      <canvas
        ref={canvasRef}
        className="canvas-2d"
        data-testid="paper-canvas"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
        onWheel={handleWheel}
        style={{
          cursor: mode === 'pan' ? 'grab' : (isDragging ? 'grabbing' : 'default'),
          touchAction: 'none'
        }}
      />
      <ZoomControls
        zoom={zoom}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onZoomFit={handleZoomFit}
      />
    </div>
  );
}

// EmptyState Component
function EmptyState({ icon: Icon, message }: { icon: any, message: string }) {
  return (
    <div className="empty-state">
      <Icon size={64} className="empty-state-icon" />
      <p>{message}</p>
    </div>
  );
}

// Main App Component
export default function App() {
  const [status, setStatus] = useState({ connected: false, hasModel: false });
  // @ts-ignore - useHistory is generic but inferred usage is complex, suppressing for speed
  const [project, setProject, undo, redo, canUndo, canRedo, resetProject] = useHistory<Project | null>(null);
  const [mode, setMode] = useState('select');
  const [selectedIslands, setSelectedIslands] = useState<number[]>([]);
  const [viewOptions, setViewOptions] = useState({
    showFlaps: true,
    showTextures: false,
  });
  const [isLoading, setIsLoading] = useState(false);
  const [, setError] = useState<string | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [hoveredEdge, setHoveredEdge] = useState<any | null>(null);

  // Check backend status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const data = await api.getStatus();
        setStatus({ connected: true, hasModel: data.has_model });
        if (data.has_model && !project) {
          const projectData = await api.getProject();
          resetProject(projectData);
        }
      } catch (err) {
        setStatus({ connected: false, hasModel: false });
      }
    };

    checkStatus();
    const interval = setInterval(checkStatus, 5000);
    return () => clearInterval(interval);
  }, [project]);

  // Handle file upload
  const handleUpload = async (file: File) => {
    setIsLoading(true);
    setError(null);
    try {
      await api.uploadModel(file);
      const projectData = await api.getProject();
      resetProject(projectData);
      setStatus(s => ({ ...s, hasModel: true }));
      setSelectedIslands([]);
    } catch (err: any) {
      setError('Failed to upload model: ' + err.message);
    } finally {
      setIsLoading(false);
    }
  };

  // Handle island selection
  const handleSelectIsland = useCallback((islandId: IslandId | null, addToSelection = false) => {
    const id = islandId?.idx ?? null;
    if (id === null) {
      setSelectedIslands([]);
    } else if (addToSelection) {
      setSelectedIslands(prev =>
        prev.includes(id)
          ? prev.filter(i => i !== id)
          : [...prev, id]
      );
    } else {
      setSelectedIslands([id]);
    }
  }, []);

  // Handle island move
  const handleMoveIsland = useCallback(async (islandId: IslandId, delta: [number, number]) => {
    try {
      // islandId is the full key object {idx, version}
      const result = await api.performAction({
        type: 'moveIsland',
        island: islandId,
        delta
      });
      setProject(result);
    } catch (err: any) {
      setError('Failed to move island: ' + err.message);
      // Revert optimization? Complex without dedicated revert
    }
  }, []);

  // Handle island rotate
  const handleRotateIsland = useCallback(async (islandId: IslandId, angle: number, center: [number, number]) => {
    try {
      const result = await api.performAction({
        type: 'rotateIsland',
        island: islandId,
        angle,
        center
      });
      setProject(result);
    } catch (err: any) {
      setError('Failed to rotate island: ' + err.message);
    }
  }, []);

  // Handle view option change
  const handleViewOptionChange = useCallback((option: string, value: boolean) => {
    setViewOptions(prev => ({ ...prev, [option]: value }));
  }, []);

  // Keyboard shortcuts for mode switching
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
        return;
      }

      switch (e.key.toLowerCase()) {
        case 'v': setMode('select'); break;
        case 'h': setMode('pan'); break;
        case 'm': setMode('move'); break;
        case 'r': setMode('rotate'); break;
        case 'c': setMode('cut'); break;
        case 'j': setMode('join'); break;
        case 'escape': setSelectedIslands([]); break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Handle options save
  const handleOptionsSave = async (newOptions: SettingsOptions) => {
    try {
      const action = api.actions.setOptions(newOptions, false);
      const updatedProject = await api.performAction(action);
      setProject(updatedProject);
    } catch (e: any) {
      setError(e.message);
    }
  };

  // Handle export
  const handleExport = (format: string) => {
    window.open(`http://localhost:3000/api/export?format=${format}`, '_blank');
  };

  return (
    <div className="app-container">
      <header className="app-header">
        <h1 className="app-title">
          <span className="app-logo"><Origami size={24} /></span>
          Papercraft Web
        </h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <FileUpload onUpload={handleUpload} isLoading={isLoading} compact />
          <StatusIndicator connected={status.connected} hasModel={status.hasModel} />
        </div>
        <div className="header-right" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <button
            className="toolbar-btn"
            onClick={() => window.open('/docs/index.html', '_blank')}
            title="Open Documentation"
          >
            <HelpCircle size={20} />
          </button>
        </div>
      </header>

      <div className="main-content">
        <div className="toolbar-container">
          <Toolbar
            mode={mode}
            onModeChange={setMode}
            viewOptions={viewOptions}
            onViewOptionChange={handleViewOptionChange}
            onOpenSettings={() => setIsSettingsOpen(true)}
            onExport={handleExport}
            onUndo={undo}
            onRedo={redo}
            canUndo={canUndo}
            canRedo={canRedo}
          />
        </div>

        <div className="content-area">
          {project ? (
            <>
              <div className="preview-pane">
                {/* 3D Preview */}
                <div className="preview-3d-container">
                  <Preview3D project={project} hoveredEdge={hoveredEdge} />
                </div>
              </div>
              <div className="canvas-pane">
                <Canvas2D
                  project={project}
                  mode={mode}
                  viewOptions={viewOptions}
                  selectedIslands={selectedIslands}
                  onSelectIsland={handleSelectIsland}
                  onMoveIsland={handleMoveIsland}
                  onRotateIsland={handleRotateIsland}
                  onProjectUpdate={(p) => setProject(p)}
                  onError={(msg) => setError(msg)}
                  hoveredEdge={hoveredEdge}
                  onHoverEdge={setHoveredEdge}
                />
              </div>
            </>
          ) : (
            <div className="empty-state-container">
              <EmptyState
                icon={Box}
                message="Upload a 3D model to start crafting"
              />
            </div>
          )}
        </div>
      </div>

      <SettingsDialog
        isOpen={isSettingsOpen}
        onOpenChange={setIsSettingsOpen}
        options={project?.options || null}
        onSave={handleOptionsSave}
      />
    </div>
  );
}
