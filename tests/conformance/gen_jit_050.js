function f50(a, b) { let v0 = 4; for (let i = 0; i !== a; i = i + 1) { v0 = b * v0; v0 = b * b; v0 = a + v0; if (v0 > 41012) v0 = v0 % 572; v0 = a * b; v0 = v0 * a; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f50(k % 7, k);
console.log(acc);
console.log(f50(5, 3));
console.log(f50(20, 2147483));
console.log(f50(3, 0.5));
console.log(f50(4, -7.25));
console.log(f50(12, 1000000.0));
console.log(f50(0, 9));
console.log(f50(30, 65536));
