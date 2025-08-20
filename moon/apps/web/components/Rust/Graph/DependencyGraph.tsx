import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
// import { useParams } from "next/navigation";

export interface GraphDependency {
    name_and_version: string;
    cve_count: number;
    direct_dependency?: GraphDependency[];
}

interface DependencyNode extends d3.SimulationNodeDatum {
    id: string;
    color: string;
    cve_count: number;
}

interface DependencyLink extends d3.SimulationLinkDatum<DependencyNode> {
    source: DependencyNode;
    target: DependencyNode;
}

// 模拟数据生成函数
const generateMockDependencyData = (): GraphDependency => {
    const dependencies: GraphDependency[] = [
        {
            name_and_version: "tokio-1.35.1",
            cve_count: 0,
            direct_dependency: [
                {
                    name_and_version: "bytes-1.5.0",
                    cve_count: 2,
                    direct_dependency: [
                        { name_and_version: "serde-1.0.195", cve_count: 1 },
                        { name_and_version: "log-0.4.20", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "futures-0.3.29",
                    cve_count: 0,
                    direct_dependency: [
                        { name_and_version: "pin-project-1.1.3", cve_count: 0 },
                        { name_and_version: "futures-core-0.3.29", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "mio-0.8.8",
                    cve_count: 5,
                    direct_dependency: [
                        { name_and_version: "libc-0.2.150", cve_count: 3 },
                        { name_and_version: "log-0.4.20", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "num_cpus-1.16.0",
                    cve_count: 0,
                    direct_dependency: [
                        { name_and_version: "hermit-abi-0.3.3", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "parking_lot-0.12.1",
                    cve_count: 8,
                    direct_dependency: [
                        { name_and_version: "lock_api-0.4.11", cve_count: 2 },
                        { name_and_version: "scopeguard-1.2.0", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "signal-hook-registry-1.4.1",
                    cve_count: 0,
                    direct_dependency: [
                        { name_and_version: "libc-0.2.150", cve_count: 3 }
                    ]
                },
                {
                    name_and_version: "socket2-0.5.5",
                    cve_count: 12,
                    direct_dependency: [
                        { name_and_version: "libc-0.2.150", cve_count: 3 },
                        { name_and_version: "winapi-0.3.9", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "tracing-0.1.40",
                    cve_count: 0,
                    direct_dependency: [
                        { name_and_version: "tracing-core-0.1.32", cve_count: 0 },
                        { name_and_version: "log-0.4.20", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "windows-sys-0.48.0",
                    cve_count: 0,
                    direct_dependency: [
                        { name_and_version: "windows-targets-0.48.5", cve_count: 0 }
                    ]
                },
                {
                    name_and_version: "serde-1.0.195",
                    cve_count: 1,
                    direct_dependency: [
                        { name_and_version: "serde_derive-1.0.195", cve_count: 0 }
                    ]
                }
            ]
        }
    ];

    return dependencies[0];
};

const DependencyGraph: React.FC = () => {
    const [graphDependencies, setGraphDependencies] = useState<GraphDependency | null>(null);
    const d3Container = useRef<HTMLDivElement | null>(null);
    // const params = useParams();

    useEffect(() => {
        // 使用本地模拟数据替代 API 调用
        const mockData = generateMockDependencyData();

        // console.log('Mock data for graph:', mockData);
        setGraphDependencies(mockData);
    }, []);

    useEffect(() => {
        if (!graphDependencies || d3Container.current === null) return;

        const containerWidth = d3Container.current.clientWidth;

        const containerHeight = d3Container.current.clientHeight;

        const width = containerWidth;

        const height = containerHeight;

        d3.select(d3Container.current).select('svg').remove();

        const svg = d3.select(d3Container.current).append('svg')
            .attr('width', '100%')
            .attr('height', '100%')
            .attr('viewBox', `0 0 ${width} ${height}`)
            .attr('preserveAspectRatio', 'xMidYMid slice');

        svg.append('defs').append('marker')
            .attr('id', 'arrowhead')
            .attr('viewBox', '0 -5 10 10')
            .attr('refX', 7)
            .attr('refY', 0)
            .attr('orient', 'auto')
            .attr('markerWidth', 4)
            .attr('markerHeight', 4)
            .append('path')
            .attr('d', 'M 0,-5 L 10,0 L 0,5')
            .attr('fill', '#333')
            .style('stroke', 'none');

        const nodesMap = new Map<string, DependencyNode>();

        const links: DependencyLink[] = [];

        // 根据cve_count设置节点颜色
        function processDependencies(dep: GraphDependency, parent?: DependencyNode) {
            const nodeId = `${dep.name_and_version}`;

            let node = nodesMap.get(nodeId);

            if (!node) {
                const getColorByCveCount = (count: number) => {

                    if (count === 0) return 'rgb(46,204,113)';  // 绿色 - CVE = 0
                    return 'rgb(229,72,77)';  // 红色 - CVE > 0
                };

                const nodeColor = !parent ? 'rgb(50,224,196)' : getColorByCveCount(dep.cve_count);

                node = {
                    id: nodeId,
                    color: nodeColor,
                    cve_count: dep.cve_count
                };
                nodesMap.set(nodeId, node);
            }
            if (parent) {
                links.push({ source: parent, target: node });
            }
            if (dep.direct_dependency) {
                dep.direct_dependency.forEach(subDep => processDependencies(subDep, node));
            }
        }

        processDependencies(graphDependencies);

        const nodes = Array.from(nodesMap.values());

        const simulation = d3.forceSimulation<DependencyNode>(nodes)

            .force('link', d3.forceLink<DependencyNode, DependencyLink>(links).id(d => d.id).distance(150))
            .force('charge', d3.forceManyBody().strength(-1000))
            .force('center', d3.forceCenter(width / 2, height / 2))
            .force('collide', d3.forceCollide().radius(20));

        const g = svg.append('g');

        const link = g.append('g')
            .selectAll('line')
            .data(links)
            .enter().append('line')
            .attr('stroke-width', 1)
            .attr('stroke', '#333')
            .attr('marker-end', 'url(#arrowhead)')
            .attr('x2', function (d) {
                const dx = (d.target as DependencyNode).x! - (d.source as DependencyNode).x!;

                const dy = (d.target as DependencyNode).y! - (d.source as DependencyNode).y!;

                const dist = Math.sqrt(dx * dx + dy * dy);

                return dist === 0 ? 0 : (d.target as DependencyNode).x! - (dx * 7 / dist);
            })
            .attr('y2', function (d) {
                const dx = (d.target as DependencyNode).x! - (d.source as DependencyNode).x!;

                const dy = (d.target as DependencyNode).y! - (d.source as DependencyNode).y!;

                const dist = Math.sqrt(dx * dx + dy * dy);
                
                return dist === 0 ? 0 : (d.target as DependencyNode).y! - (dy * 7 / dist);
            });

        const node = g.append('g')
            .selectAll('circle')
            .data(nodes)
            .enter().append('circle')
            .attr('r', 7)
            .attr('fill', d => d.color)
            .attr('stroke', '#333')
            .attr('stroke-width', 1)
            .call(d3.drag<SVGCircleElement, DependencyNode>()
                .on('start', dragstarted)
                .on('drag', dragged)
                .on('end', dragended));

        node.append('title')
            .text(d => d.id);

        const labels = g.append('g')
            .attr('class', 'labels')
            .selectAll('text')
            .data(nodes)
            .enter().append('text')
            .attr('dy', '.35em')
            .attr('x', d => d.x! + 12)
            .attr('y', d => d.y!)
            .text(d => d.id)
            .style('font-family', '"HarmonyOS Sans SC"')
            .style('font-size', '12px')
            .style('font-weight', '400')
            .style('fill', d => d.color === 'rgb(229,72,77)' ? '#e5484d' : '#000000')
            .style('text-transform', 'capitalize');

        simulation
            .nodes(nodes)
            .on('tick', ticked);

        (simulation.force('link') as d3.ForceLink<DependencyNode, DependencyLink>).links(links);

        function ticked() {
            link
                .attr('x1', d => (d.source as DependencyNode).x!)
                .attr('y1', d => (d.source as DependencyNode).y!)
                .attr('x2', function (d) {
                    const dx = (d.target as DependencyNode).x! - (d.source as DependencyNode).x!;

                    const dy = (d.target as DependencyNode).y! - (d.source as DependencyNode).y!;

                    const dist = Math.sqrt(dx * dx + dy * dy);

                    return dist === 0 ? 0 : (d.target as DependencyNode).x! - (dx * 7 / dist);
                })
                .attr('y2', function (d) {
                    const dx = (d.target as DependencyNode).x! - (d.source as DependencyNode).x!;

                    const dy = (d.target as DependencyNode).y! - (d.source as DependencyNode).y!;
                    
                    const dist = Math.sqrt(dx * dx + dy * dy);

                    return dist === 0 ? 0 : (d.target as DependencyNode).y! - (dy * 7 / dist);
                });

            node
                .attr('cx', d => d.x!)
                .attr('cy', d => d.y!);

            labels
                .attr('x', d => d.x! + 12)
                .attr('y', d => d.y!);
        }

        function dragstarted(event: d3.D3DragEvent<SVGCircleElement, DependencyNode, DependencyNode>, d: DependencyNode) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
        }

        function dragged(event: d3.D3DragEvent<SVGCircleElement, DependencyNode, DependencyNode>, d: DependencyNode) {
            d.fx = event.x;
            d.fy = event.y;
        }

        function dragended(event: d3.D3DragEvent<SVGCircleElement, DependencyNode, DependencyNode>, d: DependencyNode) {
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null;
            d.fy = null;
        }

        const zoom = d3.zoom<SVGSVGElement, unknown>()
            .scaleExtent([0.1, 20])
            .on('zoom', (event) => {
                g.attr('transform', event.transform);
            });

        svg.call(zoom);
        const initialScale = 0.8;
        
        svg.call(
            zoom.transform,
            d3.zoomIdentity
                .translate(width / 2, height / 2)
                .scale(initialScale)
                .translate(-width / 2, -height / 2)
        );

    }, [graphDependencies]);

    return (
        <div ref={d3Container} style={{
            width: '1300px',
            height: '600px',
            overflow: 'hidden'
        }} />
    );
};

export default DependencyGraph;