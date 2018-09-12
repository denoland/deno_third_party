"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var Statement_1 = require("./Statement");
exports.TryStatementBase = Statement_1.Statement;
var TryStatement = /** @class */ (function (_super) {
    tslib_1.__extends(TryStatement, _super);
    function TryStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this try statement's try block.
     */
    TryStatement.prototype.getTryBlock = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.tryBlock);
    };
    /**
     * Gets this try statement's catch clause or undefined if none exists.
     */
    TryStatement.prototype.getCatchClause = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.catchClause);
    };
    /**
     * Gets this try statement's catch clause or throws if none exists.
     */
    TryStatement.prototype.getCatchClauseOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getCatchClause(), "Expected to find a catch clause.");
    };
    /**
     * Gets this try statement's finally block or undefined if none exists.
     */
    TryStatement.prototype.getFinallyBlock = function () {
        if (this.compilerNode.finallyBlock == null || this.compilerNode.finallyBlock.getFullWidth() === 0)
            return undefined;
        return this.getNodeFromCompilerNode(this.compilerNode.finallyBlock);
    };
    /**
     * Gets this try statement's finally block or throws if none exists.
     */
    TryStatement.prototype.getFinallyBlockOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getFinallyBlock(), "Expected to find a finally block.");
    };
    return TryStatement;
}(exports.TryStatementBase));
exports.TryStatement = TryStatement;
