function f29(a, b) { let v0 = 1; let v1 = 5; for (let i = 0; i !== a; i = i + 1) { v1 = a * v1; v0 = v1 * 1; v0 = v0 - b; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f29(k % 7, k);
console.log(acc);
console.log(f29(5, 3));
console.log(f29(20, 2147483));
console.log(f29(3, 0.5));
console.log(f29(4, -7.25));
console.log(f29(12, 1000000.0));
console.log(f29(0, 9));
console.log(f29(30, 65536));
