"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var Es5PropSaver_1 = require("../Es5PropSaver");
var Es5WeakMap = /** @class */ (function () {
    function Es5WeakMap() {
        this.propSaver = new Es5PropSaver_1.Es5PropSaver();
    }
    Es5WeakMap.prototype.get = function (key) {
        return this.propSaver.get(key);
    };
    Es5WeakMap.prototype.set = function (key, value) {
        this.propSaver.set(key, value);
    };
    Es5WeakMap.prototype.has = function (key) {
        return this.propSaver.get(key) != null;
    };
    Es5WeakMap.prototype.delete = function (key) {
        this.propSaver.remove(key);
    };
    return Es5WeakMap;
}());
exports.Es5WeakMap = Es5WeakMap;
