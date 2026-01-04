import React, { useMemo } from 'react';
import { Canvas } from '@react-three/fiber';
import { OrbitControls, Center, Environment } from '@react-three/drei';
import * as THREE from 'three';
import { Project } from './types';

interface Preview3DProps {
    project: Project | null;
    hoveredEdge?: { id: number } | null;
}

const Model: React.FC<{ project: Project }> = ({ project }) => {
    const geometry = useMemo(() => {
        if (!project || !project.model) return null;

        const { vs, fs } = project.model;
        if (!vs || !fs) return null;

        const positions: number[] = [];

        // In our model structure:
        // vs is array of { p: [x,y,z], n: [x,y,z], t: [u,v] }
        // fs is array of { m: material_idx, vs: [idx0, idx1, idx2], es: [edge indices] }

        fs.forEach(face => {
            // Triangulate face if it has more than 3 vertices?
            // The current backend seems to output triangles or convex polygons.
            const indices = face.vs;

            // Fan triangulation (0,1,2), (0,2,3), etc.
            for (let i = 1; i < indices.length - 1; i++) {
                const i0 = indices[0];
                const i1 = indices[i];
                const i2 = indices[i + 1];

                const v0 = vs[i0];
                const v1 = vs[i1];
                const v2 = vs[i2];

                // Positions
                positions.push(...v0.p);
                positions.push(...v1.p);
                positions.push(...v2.p);
            }
        });

        const posAttr = new THREE.Float32BufferAttribute(positions, 3);
        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', posAttr);

        // Compute normals for lighting
        geo.computeVertexNormals();

        return geo;

    }, [project]);

    if (!geometry) return null;

    return (
        <group>
            <mesh geometry={geometry} castShadow receiveShadow>
                <meshStandardMaterial
                    color="#ffffff"
                    side={THREE.DoubleSide}
                    flatShading={true}
                    roughness={0.5}
                />
            </mesh>
            {/* Wireframe overlay */}
            <mesh geometry={geometry}>
                <meshBasicMaterial
                    color="#000000"
                    wireframe={true}
                    transparent
                    opacity={0.1}
                />
            </mesh>
        </group>
    );
};

// Component to render the highlighted edge in 3D
const HighlightedEdge: React.FC<{ project: Project; edgeId: number }> = ({ project, edgeId }) => {
    const geometry = useMemo(() => {
        if (!project?.model) return null;

        const { vs, es } = project.model;
        if (!vs || !es) return null;

        const edge = es[edgeId];
        if (!edge) return null;

        // Get vertex positions from edge
        const v0 = vs[edge.v0]?.p;
        const v1 = vs[edge.v1]?.p;
        if (!v0 || !v1) return null;

        const points = [
            new THREE.Vector3(v0[0], v0[1], v0[2]),
            new THREE.Vector3(v1[0], v1[1], v1[2])
        ];

        return new THREE.BufferGeometry().setFromPoints(points);
    }, [project, edgeId]);

    if (!geometry) return null;

    return (
        <lineSegments geometry={geometry}>
            <lineBasicMaterial color="#f59e0b" linewidth={3} />
        </lineSegments>
    );
};

export default function Preview3D({ project, hoveredEdge }: Preview3DProps) {
    return (
        <div className="w-full h-full bg-slate-900 rounded-lg overflow-hidden relative">
            <div className="absolute top-2 left-2 z-10 px-2 py-1 bg-black/50 text-white text-xs rounded pointer-events-none">
                3D Preview
            </div>

            <Canvas shadows camera={{ position: [15, 15, 15], fov: 45 }}>
                <ambientLight intensity={0.5} />
                <pointLight position={[10, 10, 10]} intensity={1} castShadow />
                <pointLight position={[-10, -10, -10]} intensity={0.5} />

                <Center>
                    {project && project.model ? (
                        <>
                            <Model project={project} />
                            {hoveredEdge && typeof hoveredEdge.id === 'number' && (
                                <HighlightedEdge project={project} edgeId={hoveredEdge.id} />
                            )}
                        </>
                    ) : (
                        <mesh>
                            <boxGeometry args={[1, 1, 1]} />
                            <meshStandardMaterial color="#444" wireframe />
                        </mesh>
                    )}
                </Center>

                <OrbitControls makeDefault />
                <Environment preset="city" />
            </Canvas>
        </div>
    );
}
