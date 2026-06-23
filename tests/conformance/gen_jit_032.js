function f32(a, b) { let v0 = 0; let v1 = 4; for (let i = 0; i !== a; i = i + 1) { v0 = v0 * 3; if (v0 > 10935) v0 = v0 % 802; v0 = v0 + a; v1 = 5 - a; if (v1 > 14593) v1 = v1 % 219; v0 = a + b; if (v0 > 15270) v0 = v0 % 306; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f32(k % 7, k);
console.log(acc);
console.log(f32(5, 3));
console.log(f32(20, 2147483));
console.log(f32(3, 0.5));
console.log(f32(4, -7.25));
console.log(f32(12, 1000000.0));
console.log(f32(0, 9));
console.log(f32(30, 65536));
