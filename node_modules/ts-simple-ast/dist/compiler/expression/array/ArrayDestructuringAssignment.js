"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var AssignmentExpression_1 = require("../AssignmentExpression");
exports.ArrayDestructuringAssignmentBase = AssignmentExpression_1.AssignmentExpression;
var ArrayDestructuringAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(ArrayDestructuringAssignment, _super);
    function ArrayDestructuringAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the left array literal expression of the array destructuring assignment.
     */
    ArrayDestructuringAssignment.prototype.getLeft = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.left);
    };
    return ArrayDestructuringAssignment;
}(exports.ArrayDestructuringAssignmentBase));
exports.ArrayDestructuringAssignment = ArrayDestructuringAssignment;
