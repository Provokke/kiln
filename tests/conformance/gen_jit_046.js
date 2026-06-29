function f46(a, b) { let v0 = 4; for (let i = 0; i < a; i = i + 1) { v0 = b + 6; v0 = a + a; v0 = a - a; v0 = b + 9; if (v0 > 6881) v0 = v0 % 966; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f46(k % 7, k);
console.log(acc);
console.log(f46(5, 3));
console.log(f46(20, 2147483));
console.log(f46(3, 0.5));
console.log(f46(4, -7.25));
console.log(f46(12, 1000000.0));
console.log(f46(0, 9));
console.log(f46(30, 65536));
