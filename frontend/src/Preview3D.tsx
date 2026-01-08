import React, { useMemo, Suspense } from 'react';
import { Canvas, useLoader } from '@react-three/fiber';
import { OrbitControls, Center, Environment } from '@react-three/drei';
import * as THREE from 'three';
import { Project, Face } from './types';

interface Preview3DProps {
    project: Project | null;
}

// Inner component that uses useLoader (requires Suspense)
const TexturedModel: React.FC<{ project: Project; textureUrls: string[]; indexMap: Map<number, number> }> = ({ project, textureUrls, indexMap }) => {
    const { model, options } = project;
    if (!model) return null;
    const { vs, fs } = model;

    const rawLoadedTextures = useLoader(THREE.TextureLoader, textureUrls);
    // useLoader returns a single texture when given one URL, or an array when given multiple
    const loadedTextures = Array.isArray(rawLoadedTextures) ? rawLoadedTextures : [rawLoadedTextures];

    const materialGroups = useMemo(() => {
        const groups = new Map<number, THREE.BufferGeometry>();
        const facesByMat: Map<number, Face[]> = new Map();
        fs.forEach(f => {
            const m = f.m;
            if (!facesByMat.has(m)) facesByMat.set(m, []);
            facesByMat.get(m)!.push(f);
        });

        facesByMat.forEach((faces, matIdx) => {
            const positions: number[] = [];
            const normals: number[] = [];
            const uvs: number[] = [];

            faces.forEach(face => {
                const indices = face.vs;
                for (let i = 1; i < indices.length - 1; i++) {
                    [indices[0], indices[i], indices[i + 1]].forEach(vIdx => {
                        const v = vs[vIdx];
                        positions.push(...v.p);
                        normals.push(...v.n);
                        // Flip V coordinate: PDO/Pepakura uses different UV convention than WebGL
                        uvs.push(v.t[0], 1.0 - v.t[1]);
                    });
                }
            });

            const geo = new THREE.BufferGeometry();
            geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
            geo.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
            geo.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
            groups.set(matIdx, geo);
        });

        return Array.from(groups.entries());
    }, [vs, fs]);

    const showTextures = options?.texture ?? true;

    // Debug: log texture info
    console.log('TexturedModel render:', {
        textureUrls,
        loadedTextures,
        indexMapEntries: Array.from(indexMap.entries()),
        showTextures
    });

    return (
        <group>
            {materialGroups.map(([matIdx, geometry]) => {
                // Use indexMap to find the correct texture for this material index
                const textureIdx = indexMap.get(matIdx);
                const texture = textureIdx !== undefined ? loadedTextures[textureIdx] : null;

                console.log(`Material ${matIdx}: textureIdx=${textureIdx}, hasTexture=${!!texture}`);

                if (texture) {
                    texture.colorSpace = THREE.SRGBColorSpace;
                    texture.wrapS = THREE.RepeatWrapping;
                    texture.wrapT = THREE.RepeatWrapping;
                    texture.needsUpdate = true;
                    if (options?.tex_filter === false) {
                        texture.minFilter = THREE.NearestFilter;
                        texture.magFilter = THREE.NearestFilter;
                    } else {
                        texture.minFilter = THREE.LinearMipmapLinearFilter;
                        texture.magFilter = THREE.LinearFilter;
                        texture.generateMipmaps = true;
                    }
                }

                return (
                    <mesh key={matIdx} geometry={geometry} castShadow receiveShadow>
                        <meshStandardMaterial
                            map={showTextures && texture ? texture : null}
                            color={showTextures && texture ? "#ffffff" : "#f0f0f0"}
                            side={THREE.DoubleSide}
                            flatShading={!showTextures}
                            roughness={0.7}
                            metalness={0.1}
                        />
                    </mesh>
                );
            })}

            {materialGroups.map(([matIdx, geometry]) => (
                <mesh key={`wire-${matIdx}`} geometry={geometry}>
                    <meshBasicMaterial
                        color="#000000"
                        wireframe={true}
                        transparent
                        opacity={0.05}
                    />
                </mesh>
            ))}
        </group>
    );
};

// Non-textured model (no useLoader, no Suspense needed)
const UntexturedModel: React.FC<{ project: Project }> = ({ project }) => {
    const { model } = project;
    if (!model) return null;
    const { vs, fs } = model;

    const geometry = useMemo(() => {
        const positions: number[] = [];
        fs.forEach(face => {
            const indices = face.vs;
            for (let i = 1; i < indices.length - 1; i++) {
                [indices[0], indices[i], indices[i + 1]].forEach(vIdx => {
                    const v = vs[vIdx];
                    positions.push(...v.p);
                });
            }
        });

        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
        geo.computeVertexNormals();
        return geo;
    }, [vs, fs]);

    return (
        <group>
            <mesh geometry={geometry} castShadow receiveShadow>
                <meshStandardMaterial
                    color="#f0f0f0"
                    side={THREE.DoubleSide}
                    flatShading={true}
                    roughness={0.7}
                    metalness={0.1}
                />
            </mesh>
            <mesh geometry={geometry}>
                <meshBasicMaterial color="#000000" wireframe={true} transparent opacity={0.05} />
            </mesh>
        </group>
    );
};

const Model: React.FC<{ project: Project }> = ({ project }) => {
    const { model } = project;
    if (!model) return null;

    // Build a list of texture URLs only for textures that have actual pixel data
    const textureData = useMemo(() => {
        const textures = model.textures || [];
        const urls: string[] = [];
        const indexMap: Map<number, number> = new Map(); // materialIndex -> loaded texture index

        textures.forEach((tex, i) => {
            if (tex.has_data) {
                indexMap.set(i, urls.length);
                urls.push(`http://localhost:3000/api/texture/${i}`);
            }
        });

        return { urls, indexMap };
    }, [model.textures]);

    // If no textures with data, render without Suspense requirements
    if (textureData.urls.length === 0) {
        return <UntexturedModel project={project} />;
    }

    return (
        <Suspense fallback={<UntexturedModel project={project} />}>
            <TexturedModel project={project} textureUrls={textureData.urls} indexMap={textureData.indexMap} />
        </Suspense>
    );
};

export default function Preview3D({ project }: Preview3DProps) {
    return (
        <div className="w-full h-full bg-slate-900 rounded-lg overflow-hidden relative">
            <div className="absolute top-2 left-2 z-10 px-2 py-1 bg-black/50 text-white text-xs rounded pointer-events-none">
                3D Preview
            </div>

            <Canvas shadows camera={{ position: [40, 40, 40], fov: 45 }}>
                <ambientLight intensity={0.5} />
                <pointLight position={[10, 10, 10]} intensity={1} castShadow />
                <pointLight position={[-10, -10, -10]} intensity={0.5} />

                <React.Suspense fallback={
                    <mesh>
                        <boxGeometry args={[1, 1, 1]} />
                        <meshStandardMaterial color="#444" wireframe />
                    </mesh>
                }>
                    <Center top>
                        {project && project.model ? (
                            <Model project={project} />
                        ) : (
                            <mesh>
                                <boxGeometry args={[1, 1, 1]} />
                                <meshStandardMaterial color="#444" wireframe />
                            </mesh>
                        )}
                    </Center>
                </React.Suspense>

                <OrbitControls makeDefault minDistance={1} maxDistance={100} />
                <Environment preset="city" />
            </Canvas>
        </div>
    );
}
