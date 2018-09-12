"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Node_1 = require("../../common/Node");
var expressioned_1 = require("../expressioned");
var manipulation_1 = require("../../../manipulation");
exports.SpreadAssignmentBase = expressioned_1.ExpressionedNode(Node_1.Node);
var SpreadAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(SpreadAssignment, _super);
    function SpreadAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Removes this property.
     */
    SpreadAssignment.prototype.remove = function () {
        manipulation_1.removeCommaSeparatedChild(this);
    };
    return SpreadAssignment;
}(exports.SpreadAssignmentBase));
exports.SpreadAssignment = SpreadAssignment;
