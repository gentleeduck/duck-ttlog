mod csv;

const SAMPLE: &str = "\
id,note,tail
1,\"hello, world\",a
2,\"line
break\",b

3,\"say \"\"hi\"\"\",c
4,ragged
";

fn main() {
  let parsed = csv::parse(SAMPLE, 4);
  println!("{:?}", parsed.headers);
  println!("{:?}", parsed.end_buffer);
  println!("chunks: {}", parsed.chunks);
}
