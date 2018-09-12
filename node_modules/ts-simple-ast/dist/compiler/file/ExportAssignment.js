"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var statement_1 = require("../statement");
var ExportAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(ExportAssignment, _super);
    function ExportAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets if this is an export equals assignemnt.
     *
     * If this is false, then it's `export default`.
     */
    ExportAssignment.prototype.isExportEquals = function () {
        return this.compilerNode.isExportEquals || false;
    };
    /**
     * Gets the export assignment expression.
     */
    ExportAssignment.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return ExportAssignment;
}(statement_1.Statement));
exports.ExportAssignment = ExportAssignment;
