"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var AssignmentExpression_1 = require("../AssignmentExpression");
exports.ObjectDestructuringAssignmentBase = AssignmentExpression_1.AssignmentExpression;
var ObjectDestructuringAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(ObjectDestructuringAssignment, _super);
    function ObjectDestructuringAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the left object literal expression of the object destructuring assignment.
     */
    ObjectDestructuringAssignment.prototype.getLeft = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.left);
    };
    return ObjectDestructuringAssignment;
}(exports.ObjectDestructuringAssignmentBase));
exports.ObjectDestructuringAssignment = ObjectDestructuringAssignment;
