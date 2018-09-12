"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function setValueIfUndefined(obj, propertyName, defaultValue) {
    if (typeof obj[propertyName] === "undefined")
        obj[propertyName] = defaultValue;
}
exports.setValueIfUndefined = setValueIfUndefined;
