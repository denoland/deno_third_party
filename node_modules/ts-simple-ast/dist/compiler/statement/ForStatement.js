"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var IterationStatement_1 = require("./IterationStatement");
exports.ForStatementBase = IterationStatement_1.IterationStatement;
var ForStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ForStatement, _super);
    function ForStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this for statement's initializer or undefined if none exists.
     */
    ForStatement.prototype.getInitializer = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.initializer);
    };
    /**
     * Gets this for statement's initializer or throws if none exists.
     */
    ForStatement.prototype.getInitializerOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getInitializer(), "Expected to find an initializer.");
    };
    /**
     * Gets this for statement's condition or undefined if none exists.
     */
    ForStatement.prototype.getCondition = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.condition);
    };
    /**
     * Gets this for statement's condition or throws if none exists.
     */
    ForStatement.prototype.getConditionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getCondition(), "Expected to find a condition.");
    };
    /**
     * Gets this for statement's incrementor.
     */
    ForStatement.prototype.getIncrementor = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.incrementor);
    };
    /**
     * Gets this for statement's incrementor or throws if none exists.
     */
    ForStatement.prototype.getIncrementorOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getIncrementor(), "Expected to find an incrementor.");
    };
    return ForStatement;
}(exports.ForStatementBase));
exports.ForStatement = ForStatement;
