use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::disk_multigraph::DiskStorage;
use crate::Edge;

#[derive(Clone, Debug)]
pub struct DiskEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes
{
    mmap_ref: &'a DiskStorage<W>,
    current_offset: u64,
    edges_left: u64,
}

impl<'a, W> DiskEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes
{
    pub fn new(mmap_ref: &'a DiskStorage<W>, offset: &u64, number_of_edges: &u64) -> DiskEdgeIterator<'a, W>{
        DiskEdgeIterator{mmap_ref, current_offset: offset.clone(), edges_left: number_of_edges.clone()}
    }
}
impl<'a, W> Iterator for DiskEdgeIterator<'a, W>
where
    W: Clone + PartialEq + FromDiskBytes{
    type Item=Edge<W>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item>{
        if self.edges_left == 0{
            return None;
        }
        
        let struct_bytes = &self.mmap_ref.mmap_structure[
            self.current_offset as usize .. self.current_offset as usize + size_of::<DiskEdge>()
        ];

        let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);
        self.current_offset += size_of::<DiskEdge>() as u64;
        
        let weight_bytes: &[u8] = &self.mmap_ref.mmap_data[
            disk_edge.weight_offset as usize.. (disk_edge.weight_offset + disk_edge.weight_len) as usize];

        let weight: W = FromDiskBytes::from_bytes(weight_bytes);

        self.edges_left-=1;

        Some(Edge::new(disk_edge.node, &weight))
    }
}

