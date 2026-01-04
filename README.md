# Papercraft Web

A web-based tool to unwrap 3D models into printable papercraft patterns. This is a client/server application with a Rust backend and React frontend.

![Papercraft Web](papercraft.svg)

## Features

- **3D Model Import**: Load OBJ, STL, and PDO files
- **Interactive Unfolding**: Cut, join, and rearrange paper pieces
- **Island Manipulation**: Move and rotate unfolded sections
- **Customizable Options**: Adjust scale, page size, flap settings
- **Export**: Generate PDF and SVG output for printing

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (1.75+)
- npm or yarn

### Installation

```bash
# Clone the repository
git clone https://github.com/rodrigorc/papercraft.git
cd papercraft

# Install JavaScript dependencies
npm install

# Build the Rust backend (first time only)
cd backend && cargo build && cd ..
```

### Development

Start both backend and frontend with hot reload:

```bash
npm run dev
```

This will:
- Start the Rust backend on `http://localhost:3000`
- Start the Vite dev server on `http://localhost:5173`
- Proxy API requests from frontend to backend

Open [http://localhost:5173](http://localhost:5173) in your browser.

### Production Build

```bash
npm run build
```

This creates an optimized frontend build in `frontend/dist/`.

## Project Structure

```
papercraft/
├── backend/           # Rust Axum API server
│   └── src/
│       ├── main.rs    # API endpoints
│       └── paper/     # Core papercraft logic
├── frontend/          # React + Vite frontend
│   └── src/
│       ├── App.jsx    # Main application
│       ├── api/       # API client
│       └── index.css  # Styles
├── tests/             # Integration tests
└── package.json       # Root package with dev scripts
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/status` | GET | Backend health check |
| `/api/upload` | POST | Upload 3D model (multipart) |
| `/api/project` | GET | Get current papercraft state |
| `/api/action` | POST | Perform actions (cut, join, move, etc.) |

### Action Types

```javascript
// Toggle flap on a cut edge
{ "type": "toggleFlap", "edge": 0, "action": "Toggle" }

// Cut an edge
{ "type": "cut", "edge": 5, "offset": null }

// Join two islands at an edge
{ "type": "join", "edge": 3, "priority_face": null }

// Move an island
{ "type": "moveIsland", "island": {...}, "delta": [10.0, 20.0] }

// Rotate an island
{ "type": "rotateIsland", "island": {...}, "angle": 0.5, "center": [100, 100] }

// Update paper options
{ "type": "setOptions", "options": {...}, "relocate_pieces": true }
```

## Running Tests

```bash
npm test
```

## Technology Stack

### Backend (Rust)
- **Axum**: Web framework
- **Tokio**: Async runtime
- **Serde**: JSON serialization
- **cgmath**: 3D math operations

### Frontend (JavaScript)
- **React 19**: UI framework
- **React Aria Components**: Accessible UI primitives
- **Vite**: Build tool and dev server
- **Tailwind CSS**: Styling

## Original Desktop Application

This web version is based on the original [Papercraft](https://github.com/rodrigorc/papercraft) desktop application by Rodrigo Rivas Costa. The core unfolding algorithms are shared between both versions.

## License

GPL-3.0-or-later - See [LICENSE](LICENSE) for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `npm test`
5. Submit a pull request

## Acknowledgments

- This project is a web port based on the original [Papercraft](https://github.com/rodrigorc/papercraft) desktop application by Rodrigo Rivas Costa.
- Original papercraft algorithms by Rodrigo Rivas Costa
- Inspired by [Pepakura Designer](https://tamasoft.co.jp/pepakura-en/)
