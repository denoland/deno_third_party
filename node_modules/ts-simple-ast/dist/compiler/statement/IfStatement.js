"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.IfStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var IfStatement = /** @class */ (function (_super) {
    tslib_1.__extends(IfStatement, _super);
    function IfStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this if statement's expression.
     */
    IfStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    /**
     * Gets this if statement's then statement.
     */
    IfStatement.prototype.getThenStatement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.thenStatement);
    };
    /**
     * Gets this if statement's else statement.
     */
    IfStatement.prototype.getElseStatement = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.elseStatement);
    };
    return IfStatement;
}(exports.IfStatementBase));
exports.IfStatement = IfStatement;
