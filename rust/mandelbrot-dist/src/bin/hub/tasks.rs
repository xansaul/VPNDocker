use crate::models::MandelbrotTask;

pub fn divide_into_chunks(
    job_id:       &str,
    total_width:  usize,
    total_height: usize,
    max_iter:     u32,
    num_chunks:   usize,
) -> Vec<MandelbrotTask> {
    let rows_per_chunk = total_height / num_chunks;
    let mut tasks = Vec::with_capacity(num_chunks);

    for i in 0..num_chunks {
        let row_start = i * rows_per_chunk;
        let row_end   = if i == num_chunks - 1 { total_height } else { (i + 1) * rows_per_chunk };

        tasks.push(MandelbrotTask {
            id:           i as u32,
            job_id:       job_id.to_string(),
            x_start:     -2.5,
            x_end:        1.0,
            y_start:     -1.2,
            y_end:        1.2,
            row_start,
            row_end,
            total_width,
            total_height,
            max_iter,
        });
    }
    tasks
}
