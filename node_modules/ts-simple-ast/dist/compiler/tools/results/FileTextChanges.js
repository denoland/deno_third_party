"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../../utils");
var TextChange_1 = require("./TextChange");
var FileTextChanges = /** @class */ (function () {
    /**
     * @internal
     */
    function FileTextChanges(compilerObject) {
        this._compilerObject = compilerObject;
    }
    /**
     * Gets the file path.
     */
    FileTextChanges.prototype.getFilePath = function () {
        return this._compilerObject.fileName;
    };
    /**
     * Gets the text changes
     */
    FileTextChanges.prototype.getTextChanges = function () {
        return this._compilerObject.textChanges.map(function (c) { return new TextChange_1.TextChange(c); });
    };
    tslib_1.__decorate([
        utils_1.Memoize
    ], FileTextChanges.prototype, "getTextChanges", null);
    return FileTextChanges;
}());
exports.FileTextChanges = FileTextChanges;
