// Allocation-heavy program; correct output proves that the GC preserves live data.
let keep = [];
for (let i = 0; i < 3000; i++) {
  const o = { id: i, tags: ["t" + i, { deep: [i, i * 2] }] };
  if (i % 100 === 0) keep.push(o);
  const garbage = [o, [o], { o }];
}
console.log(keep.length, keep[5].id, keep[29].tags[1].deep[1], keep.map(k => k.id % 7).join(""));
let list = null;
for (let i = 0; i < 2000; i++) list = { v: i, next: list };
let sum = 0; for (let n = list; n !== null; n = n.next) sum += n.v;
console.log(sum);
const old = { arr: [] };
gc();
for (let i = 0; i < 500; i++) old.arr.push({ i });  // old -> young pointers (write barrier)
gc();
console.log(old.arr.length, old.arr[499].i, old.arr.reduce((a, b) => a + b.i, 0));
