"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.BreakStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var BreakStatement = /** @class */ (function (_super) {
    tslib_1.__extends(BreakStatement, _super);
    function BreakStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this break statement's label or undefined if it does not exist.
     */
    BreakStatement.prototype.getLabel = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.label);
    };
    /**
     * Gets this break statement's label or throw if it does not exist.
     */
    BreakStatement.prototype.getLabelOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getLabel(), "Expected to find a label.");
    };
    return BreakStatement;
}(exports.BreakStatementBase));
exports.BreakStatement = BreakStatement;
