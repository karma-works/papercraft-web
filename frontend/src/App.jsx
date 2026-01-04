import { useState, useEffect, useCallback, useRef } from 'react';
import { Button, FileTrigger } from 'react-aria-components';
import { Upload, Scissors, Link2, Move, RotateCw, Settings, RefreshCw, Box, Grid3X3 } from 'lucide-react';
import * as api from './api/client';

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
function FileUpload({ onUpload, isLoading }) {
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

// Toolbar Component
function Toolbar({ mode, onModeChange, onAction }) {
  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'move' ? 'active' : ''}`}
          onClick={() => onModeChange('move')}
          title="Move Islands"
        >
          <Move size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'rotate' ? 'active' : ''}`}
          onClick={() => onModeChange('rotate')}
          title="Rotate Islands"
        >
          <RotateCw size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${mode === 'cut' ? 'active' : ''}`}
          onClick={() => onModeChange('cut')}
          title="Cut Edge"
        >
          <Scissors size={18} />
        </button>
        <button
          className={`toolbar-btn ${mode === 'join' ? 'active' : ''}`}
          onClick={() => onModeChange('join')}
          title="Join Edges"
        >
          <Link2 size={18} />
        </button>
      </div>
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={() => onAction('settings')}
          title="Settings"
        >
          <Settings size={18} />
        </button>
      </div>
    </div>
  );
}

// 2D Canvas Component for displaying the unfolded papercraft
function Canvas2D({ project, mode }) {
  const canvasRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !project) return;

    const ctx = canvas.getContext('2d');
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * window.devicePixelRatio;
    canvas.height = rect.height * window.devicePixelRatio;
    ctx.scale(window.devicePixelRatio, window.devicePixelRatio);

    // Clear canvas
    ctx.fillStyle = '#1e1e2e';
    ctx.fillRect(0, 0, rect.width, rect.height);

    // Draw grid
    ctx.strokeStyle = '#2a2a3e';
    ctx.lineWidth = 0.5;
    const gridSize = 20;
    for (let x = 0; x < rect.width; x += gridSize) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, rect.height);
      ctx.stroke();
    }
    for (let y = 0; y < rect.height; y += gridSize) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(rect.width, y);
      ctx.stroke();
    }

    // Draw paper background
    const options = project.options;
    const scale = 2; // pixels per mm
    const pageWidth = options.page_size[0] * scale;
    const pageHeight = options.page_size[1] * scale;
    const offsetX = (rect.width - pageWidth) / 2;
    const offsetY = (rect.height - pageHeight) / 2;

    ctx.fillStyle = '#ffffff';
    ctx.fillRect(offsetX, offsetY, pageWidth, pageHeight);

    // Draw islands
    const model = project.model;
    const islands = project.islands || [];

    islands.forEach((islandEntry, idx) => {
      if (!islandEntry.value) return;

      const island = islandEntry.value;
      const x = offsetX + island.x * scale;
      const y = offsetY + island.y * scale;

      // Draw island marker
      ctx.fillStyle = `hsl(${idx * 60}, 70%, 60%)`;
      ctx.beginPath();
      ctx.arc(x, y, 8, 0, Math.PI * 2);
      ctx.fill();

      // Draw island label
      ctx.fillStyle = '#1e1e2e';
      ctx.font = '10px Inter, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String.fromCharCode(65 + idx), x, y);
    });

    // Draw faces if we have model data
    if (model && model.fs && model.vs) {
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 0.5;

      // Simple face rendering - this is a placeholder
      // Real implementation would use the island transformations
    }

  }, [project, mode]);

  return (
    <canvas
      ref={canvasRef}
      className="canvas-2d"
      style={{ width: '100%', height: '100%' }}
    />
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
  const [mode, setMode] = useState('move');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);

  // Check backend status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const data = await api.getStatus();
        setStatus({ connected: true, hasModel: data.has_model });
        if (data.has_model) {
          const projectData = await api.getProject();
          setProject(projectData);
        }
      } catch (err) {
        setStatus({ connected: false, hasModel: false });
        console.error('Backend connection failed:', err);
      }
    };

    checkStatus();
    const interval = setInterval(checkStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  // Handle file upload
  const handleUpload = async (file) => {
    setIsLoading(true);
    setError(null);
    try {
      await api.uploadModel(file);
      const projectData = await api.getProject();
      setProject(projectData);
      setStatus(s => ({ ...s, hasModel: true }));
    } catch (err) {
      setError('Failed to upload model: ' + err.message);
      console.error('Upload failed:', err);
    } finally {
      setIsLoading(false);
    }
  };

  // Handle toolbar actions
  const handleAction = async (action) => {
    if (action === 'settings') {
      // TODO: Open settings panel
      console.log('Open settings');
    }
  };

  return (
    <div className="app-container">
      <header className="app-header">
        <h1 className="app-title">Papercraft Web</h1>
        <StatusIndicator connected={status.connected} hasModel={status.hasModel} />
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
          <div className="panel-header">2D Papercraft</div>
          {project && <Toolbar mode={mode} onModeChange={setMode} onAction={handleAction} />}
          <div className="panel-content">
            {project ? (
              <Canvas2D project={project} mode={mode} />
            ) : (
              <EmptyState icon={Grid3X3} message="Upload a model to see the unfolded papercraft" />
            )}
          </div>
        </div>
      </main>

      {error && (
        <div style={{
          position: 'fixed',
          bottom: '1rem',
          right: '1rem',
          background: 'var(--color-error)',
          color: 'white',
          padding: '0.75rem 1rem',
          borderRadius: '0.5rem',
          fontSize: '0.875rem'
        }}>
          {error}
        </div>
      )}
    </div>
  );
}
