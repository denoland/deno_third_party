"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * JS doc tag info.
 */
var JSDocTagInfo = /** @class */ (function () {
    /** @internal */
    function JSDocTagInfo(compilerObject) {
        this._compilerObject = compilerObject;
    }
    Object.defineProperty(JSDocTagInfo.prototype, "compilerObject", {
        /** Gets the compiler JS doc tag info. */
        get: function () {
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the name.
     */
    JSDocTagInfo.prototype.getName = function () {
        return this.compilerObject.name;
    };
    /**
     * Gets the text.
     */
    JSDocTagInfo.prototype.getText = function () {
        return this.compilerObject.text;
    };
    return JSDocTagInfo;
}());
exports.JSDocTagInfo = JSDocTagInfo;
