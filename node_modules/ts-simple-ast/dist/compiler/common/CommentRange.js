"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var errors = require("../../errors");
var CommentRange = /** @class */ (function () {
    /**
     * @internal
     */
    function CommentRange(compilerObject, sourceFile) {
        this._compilerObject = compilerObject;
        this._sourceFile = sourceFile;
    }
    Object.defineProperty(CommentRange.prototype, "compilerObject", {
        /**
         * Gets the underlying compiler object.
         */
        get: function () {
            this._throwIfForgotten();
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the source file of the comment range.
     */
    CommentRange.prototype.getSourceFile = function () {
        this._throwIfForgotten();
        return this._sourceFile;
    };
    /**
     * Gets the comment syntax kind.
     */
    CommentRange.prototype.getKind = function () {
        return this.compilerObject.kind;
    };
    /**
     * Gets the position.
     */
    CommentRange.prototype.getPos = function () {
        return this.compilerObject.pos;
    };
    /**
     * Gets the end.
     */
    CommentRange.prototype.getEnd = function () {
        return this.compilerObject.end;
    };
    /**
     * Gets the width of the comment range.
     */
    CommentRange.prototype.getWidth = function () {
        return this.getEnd() - this.getPos();
    };
    /**
     * Gets the text of the comment range.
     */
    CommentRange.prototype.getText = function () {
        var fullText = this.getSourceFile().getFullText();
        return fullText.substring(this.compilerObject.pos, this.compilerObject.end);
    };
    /**
     * Forgets the comment range.
     * @internal
     */
    CommentRange.prototype.forget = function () {
        this._compilerObject = undefined;
        this._sourceFile = undefined;
    };
    /**
     * Gets if the comment range was forgotten.
     *
     * This will be true after any manipulations have occured to the source file this comment range was generated from.
     */
    CommentRange.prototype.wasForgotten = function () {
        return this._compilerObject == null;
    };
    CommentRange.prototype._throwIfForgotten = function () {
        if (this._compilerObject != null)
            return;
        var message = "Attempted to get a comment range that was forgotten. " +
            "Comment ranges are forgotten after a manipulation has occurred. " +
            "Please re-request the comment range from the node.";
        throw new errors.InvalidOperationError(message);
    };
    return CommentRange;
}());
exports.CommentRange = CommentRange;
