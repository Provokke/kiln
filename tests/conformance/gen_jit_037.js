function f37(a, b) { let v0 = 1; let v1 = 5; let v2 = 4; for (let i = 0; i < a; i = i + 1) { v1 = a + a; v0 = v1 + 1; v2 = b * v2; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f37(k % 7, k);
console.log(acc);
console.log(f37(5, 3));
console.log(f37(20, 2147483));
console.log(f37(3, 0.5));
console.log(f37(4, -7.25));
console.log(f37(12, 1000000.0));
console.log(f37(0, 9));
console.log(f37(30, 65536));
