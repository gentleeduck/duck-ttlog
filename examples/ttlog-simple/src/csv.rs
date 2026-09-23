use std::thread;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
  Header(Vec<String>),
  Record(Vec<String>),
  NewLine,
}

#[derive(Debug)]
pub struct CSVRecord {
  pub headers: Node,
  pub end_buffer: Vec<Node>,
  pub chunks: u32,
}

#[derive(Debug)]
pub struct ThreadChunk {
  pub start: String,
  pub records: Vec<Node>,
  pub end: Option<String>,
}

pub fn parse(input: &str, threads: u8) -> CSVRecord {
  let slices = split_chunks(input, threads.max(1) as usize);

  let chunks: Vec<ThreadChunk> = thread::scope(|scope| {
    let handles: Vec<_> = slices
      .iter()
      .map(|slice| scope.spawn(move || parse_chunk(slice)))
      .collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
  });

  stitch(chunks)
}

fn split_chunks(input: &str, n: usize) -> Vec<&str> {
  let size = input.len().div_ceil(n).max(1);
  let mut out = Vec::with_capacity(n);
  let mut start = 0;

  while start < input.len() {
    let mut end = (start + size).min(input.len());
    while !input.is_char_boundary(end) {
      end += 1;
    }
    out.push(&input[start..end]);
    start = end;
  }
  out
}

fn parse_chunk(chunk: &str) -> ThreadChunk {
  let Some((body, end)) = chunk.rsplit_once('\n') else {
    return ThreadChunk {
      start: chunk.into(),
      records: Vec::new(),
      end: None,
    };
  };
  let (start, records) = match body.split_once('\n') {
    Some((start, rest)) => (start, rest.split('\n').map(node).collect()),
    None => (body, Vec::new()),
  };

  ThreadChunk {
    start: start.into(),
    records,
    end: Some(end.into()),
  }
}

fn stitch(chunks: Vec<ThreadChunk>) -> CSVRecord {
  let total = chunks.len() as u32;
  let mut nodes = Vec::new();
  let mut pending = String::new();

  for chunk in chunks {
    pending.push_str(&chunk.start);
    let Some(end) = chunk.end else { continue };
    nodes.push(node(&pending));
    nodes.extend(chunk.records);
    pending = end;
  }
  if !pending.is_empty() {
    nodes.push(node(&pending));
  }

  let at = nodes.iter().position(|n| matches!(n, Node::Record(_)));
  let headers = match at.map(|i| nodes.remove(i)) {
    Some(Node::Record(fields)) => Node::Header(fields),
    _ => Node::Header(Vec::new()),
  };

  CSVRecord {
    headers,
    end_buffer: nodes,
    chunks: total,
  }
}

fn node(record: &str) -> Node {
  match record.strip_suffix('\r').unwrap_or(record) {
    "" => Node::NewLine,
    record => Node::Record(record.split(',').map(String::from).collect()),
  }
}
