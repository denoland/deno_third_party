"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.ContinueStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var ContinueStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ContinueStatement, _super);
    function ContinueStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this continue statement's label or undefined if it does not exist.
     */
    ContinueStatement.prototype.getLabel = function () {
        return this.compilerNode.label == null
            ? undefined
            : this.getNodeFromCompilerNode(this.compilerNode.label);
    };
    /**
     * Gets this continue statement's label or throw if it does not exist.
     */
    ContinueStatement.prototype.getLabelOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getLabel(), "Expected to find a label.");
    };
    return ContinueStatement;
}(exports.ContinueStatementBase));
exports.ContinueStatement = ContinueStatement;
