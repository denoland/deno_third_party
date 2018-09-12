"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * Diagnostic message chain.
 */
var DiagnosticMessageChain = /** @class */ (function () {
    /** @internal */
    function DiagnosticMessageChain(compilerObject) {
        this._compilerObject = compilerObject;
    }
    Object.defineProperty(DiagnosticMessageChain.prototype, "compilerObject", {
        /**
         * Gets the underlying compiler object.
         */
        get: function () {
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the message text.
     */
    DiagnosticMessageChain.prototype.getMessageText = function () {
        return this.compilerObject.messageText;
    };
    /**
     * Gets th enext diagnostic message chain in the chain.
     */
    DiagnosticMessageChain.prototype.getNext = function () {
        var next = this.compilerObject.next;
        if (next == null)
            return undefined;
        return new DiagnosticMessageChain(next);
    };
    /**
     * Gets the code of the diagnostic message chain.
     */
    DiagnosticMessageChain.prototype.getCode = function () {
        return this.compilerObject.code;
    };
    /**
     * Gets the category of the diagnostic message chain.
     */
    DiagnosticMessageChain.prototype.getCategory = function () {
        return this.compilerObject.category;
    };
    return DiagnosticMessageChain;
}());
exports.DiagnosticMessageChain = DiagnosticMessageChain;
