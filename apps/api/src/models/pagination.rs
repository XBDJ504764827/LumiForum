use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

impl PaginationMeta {
    pub fn new(page: u32, page_size: u32, total: u64) -> Self {
        let page_size = page_size.max(1);
        let total_pages = total
            .div_ceil(u64::from(page_size))
            .min(u64::from(u32::MAX)) as u32;
        Self {
            page,
            page_size,
            total,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PaginationMeta;

    #[test]
    fn calculates_total_pages() {
        let pagination = PaginationMeta::new(2, 20, 41);
        assert_eq!(pagination.total_pages, 3);
    }
}
