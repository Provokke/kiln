function f9(a, b) { let v0 = 4; for (let i = 0; i < a; i = i + 1) { v0 = b * v0; v0 = v0 - b; v0 = b + 9; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f9(k % 7, k);
console.log(acc);
console.log(f9(5, 3));
console.log(f9(20, 2147483));
console.log(f9(3, 0.5));
console.log(f9(4, -7.25));
console.log(f9(12, 1000000.0));
console.log(f9(0, 9));
console.log(f9(30, 65536));
