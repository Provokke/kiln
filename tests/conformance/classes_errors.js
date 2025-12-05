class A {}
try { A(); } catch (e) { console.log(e.constructor === TypeError, e.message); }
try { const x = 1; x = 2; } catch (e) { console.log(e.name, e.message); }
try { undefinedVar + 1; } catch (e) { console.log(e.name, e.message); }
try { (void 0).foo; } catch (e) { console.log(e.message); }
try { let o = {}; o.f(); } catch (e) { console.log(e instanceof TypeError); }
