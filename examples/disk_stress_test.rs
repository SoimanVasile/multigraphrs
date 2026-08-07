use multigraphrs::{Directed, DiskStorage, MultiGraph};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Returns the current process RSS (Resident Set Size) in bytes on Linux.
/// Falls back to 0 on other platforms.
fn get_rss_bytes() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let kb: usize = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                    return kb * 1024;
                }
            }
        }
        0
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.2} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_duration(d: std::time::Duration) -> String {
    if d.as_secs() >= 1 {
        format!("{:.3}s", d.as_secs_f64())
    } else if d.as_millis() >= 1 {
        format!("{:.2}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{:.2}μs", d.as_secs_f64() * 1_000_000.0)
    }
}

fn dir_total_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

struct BenchResult {
    label: String,
    duration: std::time::Duration,
    rss_before: usize,
    rss_after: usize,
    disk_size: u64,
}

impl BenchResult {
    fn print(&self) {
        let mem_delta = self.rss_after as i64 - self.rss_before as i64;
        let sign = if mem_delta >= 0 { "+" } else { "" };
        println!(
            "  {:<40} {:>12}  |  RSS: {} → {} ({}{})  |  Disk: {}",
            self.label,
            format_duration(self.duration),
            format_bytes(self.rss_before),
            format_bytes(self.rss_after),
            sign,
            format_bytes(mem_delta.unsigned_abs() as usize),
            format_bytes(self.disk_size as usize),
        );
    }
}

fn run_stress_test(node_count: u32, edges_per_node: u32, test_name: &str) {
    let mut dir = PathBuf::from("/home/missuki/Documents/rust_temp/");
    dir.push(format!("multigraphrs_stress_{}", test_name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let backend = DiskStorage::<u32, u32>::new(&dir);
    let mut graph: multigraphrs::DiskMultiGraph<u32, u32, multigraphrs::Directed> = 
        MultiGraph::with_backend(backend);

    let total_start = Instant::now();
    let mut results: Vec<BenchResult> = Vec::new();

    // ─── Phase 1: Add Nodes ───
    let rss_before = get_rss_bytes();
    let start = Instant::now();
    graph.bulk_add_node(&Vec::from_iter(0..node_count));
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!("Add {} nodes", node_count),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // ─── Phase 2: Add Edges (chain + fan-out) ───
    let rss_before = get_rss_bytes();
    let mut edge_total = 0u64;
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(node_count as usize* edges_per_node as usize);
    for i in 0..node_count {
        for j in 1..=edges_per_node {
            let target = (i + j) % node_count;
            if target != i {
                edges.push((i, target));
                edge_total += 1;
            }
        }
    }
    let start = Instant::now();
    graph.bulk_add_edge(&edges);
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!("Add {} edges ({}/node)", edge_total, edges_per_node),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // ─── Phase 3: Query — degree for all nodes ───
    let rss_before = get_rss_bytes();
    let start = Instant::now();
    let mut degree_sum = 0usize;
    for i in 0..node_count {
        degree_sum += graph.degree(&i).unwrap();
    }
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!("Query degree (all nodes, sum={})", degree_sum),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // ─── Phase 4: Query — get_neighbours for a sample ───
    let sample_count = (node_count / 10).max(1);
    let rss_before = get_rss_bytes();
    let start = Instant::now();
    let mut neighbour_sum = 0usize;
    for i in 0..sample_count {
        let neighbours = graph.get_neighbours(&i).unwrap();
        neighbour_sum += neighbours.len();
    }
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!(
            "Get neighbours ({}% sample, sum={})",
            10, neighbour_sum
        ),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // ─── Phase 5: contains_edge checks ───
    let check_count = (node_count / 10).max(1);
    let rss_before = get_rss_bytes();
    let start = Instant::now();
    let mut found = 0u32;
    for i in 0..check_count {
        let target = (i + 1) % node_count;
        if graph.contains_edge(&i, &target).unwrap() {
            found += 1;
        }
    }
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!("Contains edge ({} checks, {} found)", check_count, found),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // ─── Phase 6: Remove edges (50% of nodes lose 1 edge each) ───
    let remove_edge_count = node_count / 2;
    let rss_before = get_rss_bytes();
    let start = Instant::now();
    // let mut removed = 0u32;
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(remove_edge_count as usize);
    for i in 0..remove_edge_count {
        let target = (i + 1) % node_count;
        edges.push((i, target));
    }
    graph.bulk_remove_edge(&edges);
    let duration = start.elapsed();
    let rss_after = get_rss_bytes();
    results.push(BenchResult {
        label: format!("Remove {} edges ", remove_edge_count),
        duration,
        rss_before,
        rss_after,
        disk_size: dir_total_size(&dir),
    });

    // // ─── Phase 7: Remove nodes (25% of total) ───
    // let remove_node_count = node_count / 4;
    // let rss_before = get_rss_bytes();
    // let start = Instant::now();
    // let mut nodes_removed = 0u32;
    // for i in 0..remove_node_count {
    //     if graph.remove_node(&i).is_ok() {
    //         nodes_removed += 1;
    //     }
    // }
    // let duration = start.elapsed();
    // let rss_after = get_rss_bytes();
    // results.push(BenchResult {
    //     label: format!("Remove {} nodes ({} ok)", remove_node_count, nodes_removed),
    //     duration,
    //     rss_before,
    //     rss_after,
    //     disk_size: dir_total_size(&dir),
    // });
    //
    // // ─── Phase 8: Iterate over entire graph ───
    // let rss_before = get_rss_bytes();
    // let start = Instant::now();
    // let mut iter_nodes = 0usize;
    // let mut iter_edges = 0usize;
    // for (_node, edges) in graph.iter() {
    //     iter_nodes += 1;
    //     iter_edges += edges.len();
    // }
    // let duration = start.elapsed();
    // let rss_after = get_rss_bytes();
    // results.push(BenchResult {
    //     label: format!(
    //         "Iterate graph ({} nodes, {} edges)",
    //         iter_nodes, iter_edges
    //     ),
    //     duration,
    //     rss_before,
    //     rss_after,
    //     disk_size: dir_total_size(&dir),
    // });
    //
    let total_elapsed = total_start.elapsed();

    // ─── Print Report ───
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║  DISK GRAPH STRESS TEST: {}",  test_name);
    println!("║  Nodes: {}  |  Edges/Node: {}  |  Total time: {}", node_count, edges_per_node, format_duration(total_elapsed));
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    for r in &results {
        r.print();
    }
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    // File breakdown
    println!("  Disk File Breakdown:");
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                println!(
                    "    {:<30} {}",
                    entry.file_name().to_string_lossy(),
                    format_bytes(meta.len() as usize)
                );
            }
        }
    }
    println!("    {:<30} {}", "TOTAL", format_bytes(dir_total_size(&dir) as usize));
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝");

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}

fn main() {
    println!("=============================================================");
    println!("         multigraphrs — Disk Graph Stress Test Suite");
    println!("=============================================================");
    println!();

    // 10M nodes, 3 edges each = 30M edges
    // With 128-byte initial capacity: ~5.5 GB disk usage (fits in 7.7 GB tmpfs)
    run_stress_test(10_000_000, 3, "massive_10m");

    println!();
    println!("All stress tests complete.");
}
