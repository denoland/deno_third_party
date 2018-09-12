"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.ReturnStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var ReturnStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ReturnStatement, _super);
    function ReturnStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this return statement's expression if it exists or throws.
     */
    ReturnStatement.prototype.getExpressionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getExpression(), "Expected to find a return expression's expression.");
    };
    /**
     * Gets this return statement's expression if it exists.
     */
    ReturnStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.expression);
    };
    return ReturnStatement;
}(exports.ReturnStatementBase));
exports.ReturnStatement = ReturnStatement;
