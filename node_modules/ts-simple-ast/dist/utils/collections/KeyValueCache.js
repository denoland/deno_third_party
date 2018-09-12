"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var ArrayUtils_1 = require("../ArrayUtils");
var Es5Map_1 = require("./Es5Map");
var KeyValueCache = /** @class */ (function () {
    function KeyValueCache() {
        if (typeof Map !== undefined)
            this.cacheItems = new Map();
        else
            this.cacheItems = new Es5Map_1.Es5Map();
    }
    KeyValueCache.prototype.getSize = function () {
        return this.cacheItems.size;
    };
    KeyValueCache.prototype.getValues = function () {
        return this.cacheItems.values();
    };
    KeyValueCache.prototype.getValuesAsArray = function () {
        return ArrayUtils_1.ArrayUtils.from(this.getValues());
    };
    KeyValueCache.prototype.getKeys = function () {
        return this.cacheItems.keys();
    };
    KeyValueCache.prototype.getEntries = function () {
        return this.cacheItems.entries();
    };
    KeyValueCache.prototype.getOrCreate = function (key, createFunc) {
        var item = this.get(key);
        if (item == null) {
            item = createFunc();
            this.set(key, item);
        }
        return item;
    };
    KeyValueCache.prototype.has = function (key) {
        return this.cacheItems.has(key);
    };
    KeyValueCache.prototype.get = function (key) {
        return this.cacheItems.get(key);
    };
    KeyValueCache.prototype.set = function (key, value) {
        this.cacheItems.set(key, value);
    };
    KeyValueCache.prototype.replaceKey = function (key, newKey) {
        if (!this.cacheItems.has(key))
            throw new Error("Key not found.");
        var value = this.cacheItems.get(key);
        this.cacheItems.delete(key);
        this.cacheItems.set(newKey, value);
    };
    KeyValueCache.prototype.removeByKey = function (key) {
        this.cacheItems.delete(key);
    };
    KeyValueCache.prototype.clear = function () {
        this.cacheItems.clear();
    };
    return KeyValueCache;
}());
exports.KeyValueCache = KeyValueCache;
