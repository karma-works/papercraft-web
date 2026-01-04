import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Button, FileTrigger, ToggleButton } from 'react-aria-components';
import {
  Upload, Scissors, Link2, Move, RotateCw, Settings,
  ZoomIn, ZoomOut, Maximize2, Box, Grid3X3,
  MousePointer2, Hand
} from 'lucide-react';
import * as api from './api/client';

// Constants
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 5;
const ZOOM_STEP = 0.1;

// Status Indicator Component
function StatusIndicator({ connected, hasModel }) {
  return (
    <div className="status-indicator">
      <span className={`status-dot ${connected ? 'connected' : 'disconnected'}`} />
      <span>{connected ? (hasModel ? 'Model loaded' : 'Ready') : 'Disconnected'}</span>
    </div>
  );
}

// File Upload Component
function FileUpload({ onUpload, isLoading, compact = false }) {
  const [isDragOver, setIsDragOver] = useState(false);

  const handleDrop = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(false);
    const file = e.dataTransfer?.files[0];
    if (file) onUpload(file);
  }, [onUpload]);

  const handleDragOver = useCallback((e) => {
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
        acceptedFileTypes={['.obj', '.stl', '.pdo']}
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
        acceptedFileTypes={['.obj', '.stl', '.pdo']}
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
function ZoomControls({ zoom, onZoomIn, onZoomOut, onZoomFit }) {
  return (
    <div className="zoom-controls">
      <button className="zoom-btn" onClick={onZoomOut} title="Zoom Out">
        <ZoomOut size={16} />
      </button>
      <span className="zoom-level">{Math.round(zoom * 100)}%</span>
      <button className="zoom-btn" onClick={onZoomIn} title="Zoom In">
        <ZoomIn size={16} />
      </button>
      <button className="zoom-btn" onClick={onZoomFit} title="Fit to View">
        <Maximize2 size={16} />
      </button>
    </div>
  );
}

// Toolbar Component
function Toolbar({ mode, onModeChange, viewOptions, onViewOptionChange }) {
  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'select' ? 'active' : ''}`}
          onClick={() => onModeChange('select')}
          title="Select (V)"
        >
          <MousePointer2 size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'pan' ? 'active' : ''}`}
          onClick={() => onModeChange('pan')}
          title="Pan (H)"
        >
          <Hand size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'move' ? 'active' : ''}`}
          onClick={() => onModeChange('move')}
          title="Move Islands (M)"
        >
          <Move size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'rotate' ? 'active' : ''}`}
          onClick={() => onModeChange('rotate')}
          title="Rotate Islands (R)"
        >
          <RotateCw size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'cut' ? 'active' : ''}`}
          onClick={() => onModeChange('cut')}
          title="Cut Edge (C)"
        >
          <Scissors size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'join' ? 'active' : ''}`}
          onClick={() => onModeChange('join')}
          title="Join Edges (J)"
        >
          <Link2 size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <ToggleButton
          className={`toolbar-btn ${viewOptions.showFlaps ? 'active' : ''}`}
          isSelected={viewOptions.showFlaps}
          onChange={() => onViewOptionChange('showFlaps', !viewOptions.showFlaps)}
          title="Toggle Flaps"
        >
          <span style={{ fontSize: '12px', fontWeight: 'bold' }}>F</span>
        </ToggleButton>
      </div>
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={() => onModeChange('settings')}
          title="Settings"
        >
          <Settings size={18} />
        </button>
      </div>
    </div>
  );
}

// Interactive 2D Canvas Component
function Canvas2D({ project, mode, selectedIslands, onSelectIsland, onMoveIsland, onRotateIsland }) {
  const canvasRef = useRef(null);
  const containerRef = useRef(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState(null);
  const [draggedIsland, setDraggedIsland] = useState(null);
  const [hoveredIsland, setHoveredIsland] = useState(null);

  // Get island data with proper structure
  const islands = useMemo(() => {
    if (!project?.islands) return [];
    return project.islands
      .map((entry, idx) => ({ ...entry, idx }))
      .filter(island => island.value !== null);
  }, [project]);

  // Calculate canvas dimensions
  const options = project?.options;
  const scale = 2; // pixels per mm base scale
  const pageWidth = options ? options.page_size[0] * scale : 210 * scale;
  const pageHeight = options ? options.page_size[1] * scale : 297 * scale;

  // Hit test - find island at position
  const hitTest = useCallback((x, y) => {
    const canvas = canvasRef.current;
    if (!canvas || !project) return null;

    const rect = canvas.getBoundingClientRect();
    const offsetX = (rect.width - pageWidth * zoom) / 2 + pan.x;
    const offsetY = (rect.height - pageHeight * zoom) / 2 + pan.y;

    // Convert screen coordinates to model coordinates
    const modelX = (x - offsetX) / zoom / scale;
    const modelY = (y - offsetY) / zoom / scale;

    // Find island at this position (simple radius check for now)
    for (const island of islands) {
      const islandX = island.value.x;
      const islandY = island.value.y;
      const dx = modelX - islandX;
      const dy = modelY - islandY;
      const distance = Math.sqrt(dx * dx + dy * dy);

      // Hit radius of 20mm
      if (distance < 20) {
        return island;
      }
    }
    return null;
  }, [project, islands, zoom, pan, pageWidth, pageHeight, scale]);

  // Handle pointer down
  const handlePointerDown = useCallback((e) => {
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (mode === 'pan' || e.button === 1) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
      e.currentTarget.setPointerCapture(e.pointerId);
      return;
    }

    const island = hitTest(x, y);

    if (mode === 'select' || mode === 'move' || mode === 'rotate') {
      if (island) {
        onSelectIsland(island.idx, e.shiftKey);
        if (mode === 'move') {
          setDraggedIsland({
            idx: island.idx,
            startX: x,
            startY: y,
            origX: island.value.x,
            origY: island.value.y
          });
          setIsDragging(true);
          e.currentTarget.setPointerCapture(e.pointerId);
        }
      } else if (!e.shiftKey) {
        onSelectIsland(null);
      }
    }
  }, [mode, pan, hitTest, onSelectIsland]);

  // Handle pointer move
  const handlePointerMove = useCallback((e) => {
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (isDragging) {
      if (draggedIsland) {
        // Moving an island
        const dx = (x - draggedIsland.startX) / zoom / scale;
        const dy = (y - draggedIsland.startY) / zoom / scale;
        // Visual feedback during drag - actual API call on release
        setDraggedIsland(prev => ({
          ...prev,
          currentX: prev.origX + dx,
          currentY: prev.origY + dy
        }));
      } else if (dragStart) {
        // Panning
        setPan({
          x: e.clientX - dragStart.x,
          y: e.clientY - dragStart.y
        });
      }
    } else {
      // Hover detection
      const island = hitTest(x, y);
      setHoveredIsland(island?.idx ?? null);
    }
  }, [isDragging, draggedIsland, dragStart, zoom, scale, hitTest]);

  // Handle pointer up
  const handlePointerUp = useCallback(async (e) => {
    if (draggedIsland && draggedIsland.currentX !== undefined) {
      // Commit the move
      const dx = draggedIsland.currentX - draggedIsland.origX;
      const dy = draggedIsland.currentY - draggedIsland.origY;
      if (Math.abs(dx) > 0.1 || Math.abs(dy) > 0.1) {
        await onMoveIsland(draggedIsland.idx, [dx, dy]);
      }
    }

    setIsDragging(false);
    setDragStart(null);
    setDraggedIsland(null);
    e.currentTarget.releasePointerCapture(e.pointerId);
  }, [draggedIsland, onMoveIsland]);

  // Handle wheel zoom
  const handleWheel = useCallback((e) => {
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

    // Draw paper border
    ctx.strokeStyle = '#4a4a5e';
    ctx.lineWidth = 1;
    ctx.strokeRect(offsetX, offsetY, pageWidth * zoom, pageHeight * zoom);

    // Draw islands
    islands.forEach((island) => {
      let islandX = island.value.x;
      let islandY = island.value.y;

      // If this island is being dragged, use the dragged position
      if (draggedIsland?.idx === island.idx && draggedIsland.currentX !== undefined) {
        islandX = draggedIsland.currentX;
        islandY = draggedIsland.currentY;
      }

      const x = offsetX + islandX * scale * zoom;
      const y = offsetY + islandY * scale * zoom;

      const isSelected = selectedIslands.includes(island.idx);
      const isHovered = hoveredIsland === island.idx;

      // Draw island marker
      const radius = (isSelected ? 12 : 8) * zoom;
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);

      // Fill
      if (isSelected) {
        ctx.fillStyle = '#6366f1';
      } else if (isHovered) {
        ctx.fillStyle = '#a78bfa';
      } else {
        ctx.fillStyle = `hsl(${island.idx * 60}, 70%, 60%)`;
      }
      ctx.fill();

      // Selection ring
      if (isSelected) {
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.stroke();

        // Rotation handle (if in rotate mode)
        if (mode === 'rotate') {
          const handleDist = 30 * zoom;
          const handleX = x + handleDist;
          const handleY = y;

          ctx.beginPath();
          ctx.moveTo(x + radius, y);
          ctx.lineTo(handleX - 6, handleY);
          ctx.strokeStyle = '#6366f1';
          ctx.lineWidth = 2;
          ctx.stroke();

          ctx.beginPath();
          ctx.arc(handleX, handleY, 6, 0, Math.PI * 2);
          ctx.fillStyle = '#6366f1';
          ctx.fill();
          ctx.strokeStyle = '#ffffff';
          ctx.lineWidth = 1;
          ctx.stroke();
        }
      }

      // Draw island label
      ctx.fillStyle = isSelected ? '#ffffff' : '#1e1e2e';
      ctx.font = `${10 * zoom}px Inter, sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String.fromCharCode(65 + (island.idx % 26)), x, y);
    });

  }, [project, islands, zoom, pan, selectedIslands, hoveredIsland, draggedIsland, mode, pageWidth, pageHeight, scale]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.target.tagName === 'INPUT') return;

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

// Empty State Component
function EmptyState({ icon: Icon, message }) {
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
  const [project, setProject] = useState(null);
  const [mode, setMode] = useState('select');
  const [selectedIslands, setSelectedIslands] = useState([]);
  const [viewOptions, setViewOptions] = useState({
    showFlaps: true,
    showTextures: false,
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);

  // Check backend status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const data = await api.getStatus();
        setStatus({ connected: true, hasModel: data.has_model });
        if (data.has_model && !project) {
          const projectData = await api.getProject();
          setProject(projectData);
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
  const handleUpload = async (file) => {
    setIsLoading(true);
    setError(null);
    try {
      await api.uploadModel(file);
      const projectData = await api.getProject();
      setProject(projectData);
      setStatus(s => ({ ...s, hasModel: true }));
      setSelectedIslands([]);
    } catch (err) {
      setError('Failed to upload model: ' + err.message);
    } finally {
      setIsLoading(false);
    }
  };

  // Handle island selection
  const handleSelectIsland = useCallback((islandIdx, addToSelection = false) => {
    if (islandIdx === null) {
      setSelectedIslands([]);
    } else if (addToSelection) {
      setSelectedIslands(prev =>
        prev.includes(islandIdx)
          ? prev.filter(i => i !== islandIdx)
          : [...prev, islandIdx]
      );
    } else {
      setSelectedIslands([islandIdx]);
    }
  }, []);

  // Handle island move
  const handleMoveIsland = useCallback(async (islandIdx, delta) => {
    try {
      const island = project.islands[islandIdx];
      const result = await api.performAction({
        type: 'moveIsland',
        island: { idx: islandIdx, version: island.version },
        delta
      });
      setProject(result);
    } catch (err) {
      setError('Failed to move island: ' + err.message);
    }
  }, [project]);

  // Handle island rotate
  const handleRotateIsland = useCallback(async (islandIdx, angle, center) => {
    try {
      const island = project.islands[islandIdx];
      const result = await api.performAction({
        type: 'rotateIsland',
        island: { idx: islandIdx, version: island.version },
        angle,
        center
      });
      setProject(result);
    } catch (err) {
      setError('Failed to rotate island: ' + err.message);
    }
  }, [project]);

  // Handle view option change
  const handleViewOptionChange = useCallback((option, value) => {
    setViewOptions(prev => ({ ...prev, [option]: value }));
  }, []);

  // Keyboard shortcuts for mode switching
  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.target.tagName === 'INPUT') return;

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

  return (
    <div className="app-container">
      <header className="app-header">
        <h1 className="app-title">Papercraft Web</h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <FileUpload onUpload={handleUpload} isLoading={isLoading} compact />
          <StatusIndicator connected={status.connected} hasModel={status.hasModel} />
        </div>
      </header>

      <main className="app-main">
        {/* Left Panel - 3D View */}
        <div className="panel">
          <div className="panel-header">3D Preview</div>
          <div className="panel-content">
            {project ? (
              <EmptyState icon={Box} message="3D preview coming soon" />
            ) : (
              <FileUpload onUpload={handleUpload} isLoading={isLoading} />
            )}
          </div>
        </div>

        {/* Right Panel - 2D Papercraft View */}
        <div className="panel">
          <div className="panel-header">
            2D Papercraft
            {selectedIslands.length > 0 && (
              <span className="selection-count">
                {selectedIslands.length} selected
              </span>
            )}
          </div>
          {project && (
            <Toolbar
              mode={mode}
              onModeChange={setMode}
              viewOptions={viewOptions}
              onViewOptionChange={handleViewOptionChange}
            />
          )}
          <div className="panel-content">
            {project ? (
              <Canvas2D
                project={project}
                mode={mode}
                selectedIslands={selectedIslands}
                onSelectIsland={handleSelectIsland}
                onMoveIsland={handleMoveIsland}
                onRotateIsland={handleRotateIsland}
              />
            ) : (
              <EmptyState icon={Grid3X3} message="Upload a model to see the unfolded papercraft" />
            )}
          </div>
        </div>
      </main>

      {error && (
        <div className="error-toast" onClick={() => setError(null)}>
          {error}
        </div>
      )}
    </div>
  );
}
