"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var DirectoryEmitResult = /** @class */ (function () {
    /** @internal */
    function DirectoryEmitResult(_emitSkipped, _outputFilePaths) {
        this._emitSkipped = _emitSkipped;
        this._outputFilePaths = _outputFilePaths;
    }
    /**
     * Gets if the emit was skipped.
     */
    DirectoryEmitResult.prototype.getEmitSkipped = function () {
        return this._emitSkipped;
    };
    /**
     * Gets the output file paths.
     */
    DirectoryEmitResult.prototype.getOutputFilePaths = function () {
        return this._outputFilePaths;
    };
    return DirectoryEmitResult;
}());
exports.DirectoryEmitResult = DirectoryEmitResult;
